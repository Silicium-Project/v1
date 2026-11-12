package server

import (
	"net/http"
	"os"
	"path"
	"path/filepath"
	"strings"
	"time"
)

// largeWriteTimeout is the time allowed to send one big file: a search model (470 Mo) or an
// installer (108 Mo AppImage). The server's general write timeout is one minute, too short for
// them on a normal connection: the stream is then cut (HTTP/2 PROTOCOL_ERROR, curl exit 92).
const largeWriteTimeout = 30 * time.Minute

// spaHandler serves the built web app: real files as they are (long cache for hashed
// assets), every other path gets index.html so client-side routes survive a reload.
func spaHandler(dir string) http.Handler {
	files := http.FileServer(http.Dir(dir))
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		clean := path.Clean("/" + r.URL.Path)
		if info, err := os.Stat(filepath.Join(dir, filepath.FromSlash(clean))); err == nil && !info.IsDir() {
			if strings.HasPrefix(clean, "/download") {
				// Paquets et modèles : envoi lent autorisé (voir largeWriteTimeout).
				_ = http.NewResponseController(w).SetWriteDeadline(time.Now().Add(largeWriteTimeout))
			}
			switch {
			case strings.HasPrefix(clean, "/assets/"):
				w.Header().Set("Cache-Control", "public, max-age=31536000, immutable")
			case strings.HasPrefix(clean, "/download/models/"):
				// Fichiers identifiés par leur empreinte : cache long.
				w.Header().Set("Cache-Control", "public, max-age=604800")
			default:
				w.Header().Set("Cache-Control", "no-cache")
			}
			files.ServeHTTP(w, r)
			return
		}
		w.Header().Set("Cache-Control", "no-cache")
		http.ServeFile(w, r, filepath.Join(dir, "index.html"))
	})
}
