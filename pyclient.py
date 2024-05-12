import asyncio
import websockets
from pathlib import Path
import items_pb2

async def producer():
    return await asyncio.get_event_loop().run_in_executor(None, lambda: input("Enter something: "))

myItem = items_pb2.Information()

async def send_command_and_listen():
    uri = "ws://127.0.0.1:8080/ws"
    async with websockets.connect(uri) as websocket:
        while True: 
            msg = await producer()
            await websocket.send(msg)
            print("Sent command: JI")

            response = await websocket.recv()
            library = items_pb2.Information()
            library.ParseFromString(response)
            print("Received:", library)

asyncio.get_event_loop().run_until_complete(send_command_and_listen())

