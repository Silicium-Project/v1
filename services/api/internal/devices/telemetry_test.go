package devices

import (
	"crypto/ed25519"
	"encoding/hex"
	"fmt"
	"testing"
	"time"
)

// linkVector links the shared test-vector machine to "user-1" and returns its key.
func linkVector(t *testing.T, svc *Service) ed25519.PrivateKey {
	t.Helper()
	code, _, err := svc.NewCode("user-1")
	if err != nil {
		t.Fatal(err)
	}
	pub, sig := sign(vectorKey(), code)
	if _, err := svc.Link(code, pub, sig, "laptop"); err != nil {
		t.Fatal(err)
	}
	return vectorKey()
}

// signedReport builds a payload and its signature the way the node does: the context followed by
// the exact payload bytes, signed with the device key.
func signedReport(key ed25519.PrivateKey, observed time.Time, extra string) (pub, payload, sig string) {
	payload = fmt.Sprintf(`{"observed_at_ms":%d,"version":"0.1.0","cpu_pct":31.2,"mem_pct":46.0,`+
		`"cpu_cores":8,"roles":{"compute":true,"verify":false},"tasks_active":2,`+
		`"tasks_in_progress":1,"tasks_finalized":128,"tasks_other":1%s}`, observed.UnixMilli(), extra)
	pub = hex.EncodeToString(key.Public().(ed25519.PublicKey))
	sig = hex.EncodeToString(ed25519.Sign(key, []byte(TelemetryContext+payload)))
	return pub, payload, sig
}

func TestAMachineReportIsKeptForItsDevice(t *testing.T) {
	svc, clock := setup(t)
	key := linkVector(t, svc)
	pub, payload, sig := signedReport(key, *clock, "")
	if err := svc.ReportTelemetry(pub, payload, sig); err != nil {
		t.Fatalf("report refused: %v", err)
	}
	device := svc.List("user-1")[0]
	if device.Telemetry == nil || device.Telemetry.CPUPct != 31.2 || device.Telemetry.TasksFinalized != 128 {
		t.Fatalf("telemetry not kept: %+v", device.Telemetry)
	}
	if !device.Telemetry.Compute || device.Telemetry.Verify {
		t.Fatalf("roles not kept: %+v", device.Telemetry)
	}
}

func TestAReportWithAWrongSignatureIsRefused(t *testing.T) {
	svc, clock := setup(t)
	key := linkVector(t, svc)
	pub, payload, _ := signedReport(key, *clock, "")
	_, _, other := signedReport(key, clock.Add(time.Second), "")
	if err := svc.ReportTelemetry(pub, payload, other); err != ErrInvalidProof {
		t.Fatalf("err = %v, want ErrInvalidProof", err)
	}
}

func TestAnOlderReplayCannotRollTheStateBack(t *testing.T) {
	svc, clock := setup(t)
	key := linkVector(t, svc)
	pub, payload, sig := signedReport(key, *clock, "")
	if err := svc.ReportTelemetry(pub, payload, sig); err != nil {
		t.Fatal(err)
	}
	if err := svc.ReportTelemetry(pub, payload, sig); err != ErrStaleReport {
		t.Fatalf("replay err = %v, want ErrStaleReport", err)
	}
}

func TestAnUnknownOrRevokedDeviceIsRefused(t *testing.T) {
	svc, clock := setup(t)
	key := linkVector(t, svc)
	stranger := ed25519.NewKeyFromSeed(make([]byte, ed25519.SeedSize))
	pub, payload, sig := signedReport(stranger, *clock, "")
	if err := svc.ReportTelemetry(pub, payload, sig); err != ErrUnknownDevice {
		t.Fatalf("unknown key err = %v, want ErrUnknownDevice", err)
	}
	device := svc.List("user-1")[0]
	if err := svc.Revoke("user-1", device.ID); err != nil {
		t.Fatal(err)
	}
	pub, payload, sig = signedReport(key, clock.Add(time.Second), "")
	if err := svc.ReportTelemetry(pub, payload, sig); err != ErrUnknownDevice {
		t.Fatalf("revoked err = %v, want ErrUnknownDevice", err)
	}
}

func TestAClockFarFromOursIsRefusedAndUnknownFieldsToo(t *testing.T) {
	svc, clock := setup(t)
	key := linkVector(t, svc)
	pub, payload, sig := signedReport(key, clock.Add(time.Hour), "")
	if err := svc.ReportTelemetry(pub, payload, sig); err != ErrInvalidInput {
		t.Fatalf("far clock err = %v, want ErrInvalidInput", err)
	}
	pub, payload, sig = signedReport(key, *clock, `,"surprise":1`)
	if err := svc.ReportTelemetry(pub, payload, sig); err != ErrInvalidInput {
		t.Fatalf("unknown field err = %v, want ErrInvalidInput", err)
	}
}
