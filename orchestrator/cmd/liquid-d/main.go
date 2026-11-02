package main

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	fmt.Println("Liquid Harness Orchestrator (Go) initializing...")
	<-ctx.Done()
	fmt.Println("Shutting down Liquid Harness Orchestrator.")
}
