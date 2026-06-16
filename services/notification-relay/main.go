// Command notification-relay runs a concurrent webhook fan-out HTTP service.
//
// Configuration (environment variables):
//
//	NOTIFY_WEBHOOKS   comma-separated webhook URLs (required)
//	NOTIFY_ADDR       listen address (default ":8085")
//	NOTIFY_TIMEOUT    per-target attempt timeout, e.g. "5s" (default 5s)
//	NOTIFY_RETRIES    additional attempts after first failure (default 2)
//	NOTIFY_BACKOFF    base backoff between attempts, e.g. "250ms" (default 250ms)
//
// Endpoints:
//
//	POST /notify   fan out a JSON alert body to all targets -> delivery report
//	GET  /health   liveness + configured targets
package main

import (
	"context"
	"errors"
	"log"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"syscall"
	"time"

	"guardrail/notification-relay/relay"
)

func main() {
	if err := run(); err != nil {
		log.Fatalf("notification-relay: %v", err)
	}
}

func run() error {
	cfg, addr, err := configFromEnv()
	if err != nil {
		return err
	}

	r, err := relay.New(cfg)
	if err != nil {
		return err
	}

	srv := &http.Server{
		Addr:              addr,
		Handler:           relay.NewServer(r),
		ReadHeaderTimeout: 5 * time.Second,
		ReadTimeout:       15 * time.Second,
		WriteTimeout:      90 * time.Second,
		IdleTimeout:       60 * time.Second,
	}

	// Graceful shutdown on SIGINT/SIGTERM.
	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	errCh := make(chan error, 1)
	go func() {
		log.Printf("notification-relay listening on %s with %d target(s)", addr, len(r.Targets()))
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			errCh <- err
		}
	}()

	select {
	case err := <-errCh:
		return err
	case <-ctx.Done():
		log.Print("notification-relay shutting down")
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		return srv.Shutdown(shutdownCtx)
	}
}

func configFromEnv() (relay.Config, string, error) {
	webhooks := os.Getenv("NOTIFY_WEBHOOKS")
	if webhooks == "" {
		return relay.Config{}, "", errors.New("NOTIFY_WEBHOOKS is required (comma-separated webhook URLs)")
	}
	targets, err := relay.ParseTargets(webhooks)
	if err != nil {
		return relay.Config{}, "", err
	}

	cfg := relay.Config{Targets: targets}

	if v := os.Getenv("NOTIFY_TIMEOUT"); v != "" {
		d, err := time.ParseDuration(v)
		if err != nil {
			return relay.Config{}, "", errors.New("invalid NOTIFY_TIMEOUT: " + err.Error())
		}
		cfg.Timeout = d
	}
	if v := os.Getenv("NOTIFY_RETRIES"); v != "" {
		n, err := strconv.Atoi(v)
		if err != nil {
			return relay.Config{}, "", errors.New("invalid NOTIFY_RETRIES: " + err.Error())
		}
		cfg.Retries = n
	}
	if v := os.Getenv("NOTIFY_BACKOFF"); v != "" {
		d, err := time.ParseDuration(v)
		if err != nil {
			return relay.Config{}, "", errors.New("invalid NOTIFY_BACKOFF: " + err.Error())
		}
		cfg.Backoff = d
	}

	addr := os.Getenv("NOTIFY_ADDR")
	if addr == "" {
		addr = ":8085"
	}

	return cfg, addr, nil
}
