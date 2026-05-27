"""gRPC server implementation."""

import asyncio
import logging
from typing import Optional

import grpc

from analytics_server.service import AnalyticsServiceImpl


class AnalyticsServer:
    """Analytics gRPC server."""

    def __init__(self, address: str):
        """Initialize server.

        Args:
            address: Server address (e.g., "127.0.0.1:50051" or "unix:/tmp/analytics.sock")
        """
        self.address = address
        self.server: Optional[grpc.aio.Server] = None
        self.logger = logging.getLogger(__name__)

    async def start(self) -> None:
        """Start the gRPC server."""
        self.server = grpc.aio.server()

        # Add service implementation
        from analytics_server.analytics_pb2_grpc import add_AnalyticsServiceServicer_to_server

        add_AnalyticsServiceServicer_to_server(AnalyticsServiceImpl(), self.server)

        # Bind to address
        if self.address.startswith("unix:"):
            socket_path = self.address[5:]
            self.server.add_insecure_port(f"unix:{socket_path}")
            self.logger.info(f"Server bound to Unix socket: {socket_path}")
        else:
            self.server.add_insecure_port(self.address)
            self.logger.info(f"Server bound to TCP: {self.address}")

        await self.server.start()

    async def stop(self) -> None:
        """Stop the gRPC server gracefully."""
        if self.server:
            self.logger.info("Stopping server...")
            await self.server.stop(grace=5.0)
            self.logger.info("Server stopped")

    async def wait_for_termination(self) -> None:
        """Wait for server termination."""
        if self.server:
            await self.server.wait_for_termination()
