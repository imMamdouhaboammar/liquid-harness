package main

import (
	"context"
	"net"
	"time"

	liquidv1 "github.com/liquid-harness/liquid-harness/orchestrator/gen/liquid/v1"
	"github.com/liquid-harness/liquid-harness/orchestrator/pkg/registry"
	"go.uber.org/fx"
	"go.uber.org/zap"
	"google.golang.org/grpc"
)

func newLogger() (*zap.Logger, error) {
	return zap.NewProduction()
}

func startGrpcServer(
	lc fx.Lifecycle,
	server *registry.Server,
	scheduler *registry.ScoutScheduler,
	logger *zap.Logger,
) *grpc.Server {
	grpcServer := grpc.NewServer()
	liquidv1.RegisterToolRegistryServiceServer(grpcServer, server)

	lc.Append(fx.Hook{
		OnStart: func(ctx context.Context) error {
			// Hardened strictly to local loopback interface
			lis, err := net.Listen("tcp", "127.0.0.1:50052")
			if err != nil {
				logger.Error("Failed to bind gRPC port 50052", zap.Error(err))
				return err
			}

			logger.Info("Liquid Harness Go Registry running on 127.0.0.1:50052")
			go func() {
				if err := grpcServer.Serve(lis); err != nil {
					logger.Warn("gRPC server terminated", zap.Error(err))
				}
			}()

			// Start 24h cron background scheduler
			scheduler.Start(24 * time.Hour)
			return nil
		},
		OnStop: func(ctx context.Context) error {
			logger.Info("Stopping Liquid Harness Go Registry")
			scheduler.Stop()
			grpcServer.GracefulStop()
			return nil
		},
	})

	return grpcServer
}

func main() {
	app := fx.New(
		fx.Provide(
			newLogger,
			registry.NewRegistry,
			registry.NewServer,
			registry.NewScoutScheduler,
		),
		fx.Invoke(startGrpcServer),
	)

	app.Run()
}
