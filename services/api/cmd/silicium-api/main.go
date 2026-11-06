// Command silicium-api is the account and job service in front of a Silicium node.
package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"sil/api/internal/auth"
	"sil/api/internal/config"
	"sil/api/internal/devices"
	"sil/api/internal/jobs"
	"sil/api/internal/nodeclient"
	"sil/api/internal/server"
	"sil/api/internal/store"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "silicium-api:", err)
		os.Exit(1)
	}
}

func run() error {
	cfg, err := config.Load(os.Getenv)
	if err != nil {
		return err
	}
	log := newLogger(cfg.LogJSON)
	data, err := store.Open(cfg.StorePath)
	if err != nil {
		return fmt.Errorf("opening the store: %w", err)
	}
	authConfig := auth.DefaultConfig()
	authConfig.SessionTTL = cfg.SessionTTL
	node := nodeclient.New(cfg.NodeURL, cfg.NodeToken, cfg.NodeTimeout)
	api := server.New(
		data,
		auth.New(data, authConfig, nil),
		devices.New(data, nil),
		jobs.New(data, node, nil),
		server.Options{AllowRegistration: cfg.AllowRegistration, WebDir: cfg.WebDir, CORSOrigins: cfg.CORSOrigins, Committee: cfg.Committee, Log: log},
	)
	httpServer := &http.Server{
		Addr:              cfg.Addr,
		Handler:           api.Handler(),
		ReadHeaderTimeout: 5 * time.Second,
		ReadTimeout:       2 * time.Minute, // content uploads can be large
		WriteTimeout:      time.Minute,
		IdleTimeout:       time.Minute,
		MaxHeaderBytes:    16 << 10,
	}
	stop, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer cancel()
	failed := make(chan error, 1)
	go func() { failed <- httpServer.ListenAndServe() }()
	log.Info("listening", "addr", cfg.Addr, "node", cfg.NodeURL, "registration", cfg.AllowRegistration)
	select {
	case err := <-failed:
		return err
	case <-stop.Done():
	}
	log.Info("shutting down")
	ctx, done := context.WithTimeout(context.Background(), 15*time.Second)
	defer done()
	if err := httpServer.Shutdown(ctx); err != nil && !errors.Is(err, http.ErrServerClosed) {
		return err
	}
	return nil
}

func newLogger(json bool) *slog.Logger {
	if json {
		return slog.New(slog.NewJSONHandler(os.Stderr, nil))
	}
	return slog.New(slog.NewTextHandler(os.Stderr, nil))
}
