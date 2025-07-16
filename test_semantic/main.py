import asyncio
from async_utils import fetch_data

async def main():
    data = await fetch_data("test")
    print(data)
EOF < /dev/null