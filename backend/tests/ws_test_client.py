import asyncio
import json
from typing import Any
from typing import Callable
from typing import Dict
from typing import List

import websockets


class WebSocketTestClient:
    """
    Test client for WebSocket connections.

    This client provides a simple interface for connecting to a WebSocket server,
    sending messages, and receiving responses for testing purposes.
    """

    def __init__(self, base_url: str = "ws://localhost:8000"):
        self.base_url = base_url
        self.connection = None
        self.connected = False

    async def connect(self, path: str = "/api/ws"):
        """Connect to the WebSocket server at the specified path"""
        full_url = f"{self.base_url}{path}"
        self.connection = await websockets.connect(full_url)
        self.connected = True
        return self.connection

    async def disconnect(self):
        """Disconnect from the WebSocket server"""
        if self.connection and self.connected:
            await self.connection.close()
            self.connected = False

    async def send_json(self, data: Dict[str, Any]):
        """Send a JSON message to the WebSocket server"""
        if not self.connected or not self.connection:
            raise RuntimeError("Not connected to a WebSocket server")

        await self.connection.send(json.dumps(data))

    async def receive_json(self, timeout: float = 2.0) -> Dict[str, Any]:
        """Receive a JSON message from the WebSocket server with timeout"""
        if not self.connected or not self.connection:
            raise RuntimeError("Not connected to a WebSocket server")

        try:
            # Set a timeout to avoid hanging tests
            response = await asyncio.wait_for(self.connection.recv(), timeout=timeout)
            return json.loads(response)
        except asyncio.TimeoutError:
            raise TimeoutError(f"No message received within {timeout} seconds")


async def connect_clients(base_url: str, path: str, num_clients: int) -> List[WebSocketTestClient]:
    """
    Connect multiple WebSocket clients to the server.

    Args:
        base_url: Base URL of the WebSocket server
        path: Path to connect to
        num_clients: Number of clients to connect

    Returns:
        List of connected WebSocketTestClient instances
    """
    clients = [WebSocketTestClient(base_url) for _ in range(num_clients)]

    # Connect all clients
    for client in clients:
        await client.connect(path)

    return clients


async def disconnect_clients(clients: List[WebSocketTestClient]):
    """
    Disconnect multiple WebSocket clients.

    Args:
        clients: List of WebSocketTestClient instances to disconnect
    """
    for client in clients:
        await client.disconnect()


async def with_client(base_url: str, path: str, callback: Callable):
    """
    Context manager-like function for working with a WebSocket client.

    Args:
        base_url: Base URL of the WebSocket server
        path: Path to connect to
        callback: Async function to call with the connected client
    """
    client = WebSocketTestClient(base_url)
    try:
        await client.connect(path)
        await callback(client)
    finally:
        await client.disconnect()
