import asyncio

async def test_func():
    await asyncio.sleep(1)
    return "done"

async def main():
    result = await test_func()
    print(result)