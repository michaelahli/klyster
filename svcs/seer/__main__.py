"""Main entry point for analytics server."""

import argparse
import asyncio
import logging
import signal
import sys
from pathlib import Path

from seer.server import AnalyticsServer


def setup_logging(log_level: str) -> None:
    """Configure structured logging to stdout."""
    logging.basicConfig(
        level=getattr(logging, log_level.upper()),
        format='{"timestamp":"%(asctime)s","level":"%(levelname)s","message":"%(message)s","module":"%(name)s"}',
        datefmt="%Y-%m-%dT%H:%M:%S",
        stream=sys.stdout,
    )


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Klyster Seer (analytics sidecar)")
    parser.add_argument(
        "--socket",
        type=str,
        help="Unix socket path (e.g., /tmp/analytics.sock)",
    )
    parser.add_argument(
        "--host",
        type=str,
        default="127.0.0.1",
        help="TCP host to bind (default: 127.0.0.1)",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=50051,
        help="TCP port to bind (default: 50051)",
    )
    parser.add_argument(
        "--log-level",
        type=str,
        default="info",
        choices=["debug", "info", "warning", "error"],
        help="Log level (default: info)",
    )
    return parser.parse_args()


async def main() -> None:
    """Main entry point."""
    args = parse_args()
    setup_logging(args.log_level)

    logger = logging.getLogger(__name__)
    logger.info("Starting Klyster Seer (analytics sidecar)")

    # Determine connection type
    if args.socket:
        address = f"unix:{args.socket}"
        logger.info(f"Using Unix socket: {args.socket}")
    else:
        address = f"{args.host}:{args.port}"
        logger.info(f"Using TCP: {address}")

    # Create and start server
    server = AnalyticsServer(address)

    # Setup signal handlers for graceful shutdown
    loop = asyncio.get_event_loop()

    def signal_handler(sig):
        logger.info(f"Received signal {sig}, initiating graceful shutdown")
        asyncio.create_task(server.stop())

    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, lambda s=sig: signal_handler(s))

    try:
        await server.start()
        logger.info("Seer started successfully")
        await server.wait_for_termination()
    except Exception as e:
        logger.error(f"Server error: {e}", exc_info=True)
        sys.exit(1)
    finally:
        logger.info("Seer stopped")


if __name__ == "__main__":
    asyncio.run(main())
