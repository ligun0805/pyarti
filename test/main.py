import asyncio
from pyarti import PyArtiClient, PyArtiHSClient

# This is a test function
async def client_test():
    py_arti = PyArtiClient()
    
    try:
        py_arti.init()

        # Create firsthop circuit to the guard relay
        print("Creating firsthop circuit...")
        py_arti.create(
            "88.198.35.49",
            443,
            "ED9A731373456FA071C12A3E63E2C8BEF0A6E721"
        )

        # Extend the circuit to the middle relay and the exit relay
        print("\nExtending the circuit...")
        py_arti.extend(
            "38.152.218.16",
            443,
            "B2708B9EFA3288656DFA9750B0FB926EB811EA77",
        )
        print("\nExtending the circuit...")
        py_arti.extend(
            "185.220.100.241",
            9000,
            "62F4994C6F3A5B3E590AEECE522591696C8DDEE2"
        )

        # Connect to the target
        print("\nConnecting to the target...")
        response = py_arti.connect(
            "https://example.com",
            80,
        )
        print(response)
            
    except Exception as e:
        print(e)
        return

async def hs_client_test():
    py_arti = PyArtiHSClient()
    
    try:
        storage = {
            "state_dir": "/home/michael/Documents/Arti/state",
            "cache_dir": "/home/michael/Documents/Arti/cache"
        }
        
        py_arti.init(storage)

        py_arti.set_custom_hs_relay_ids(
            "FFA72BD683BC2FCF988356E6BEC1E490F313FB07",
            "B2708B9EFA3288656DFA9750B0FB926EB811EA77",
            "8929AF5554BE622DE3FE34812C03D65FE7D5D0F1",
        )

        duckduckgo_addr = "duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion"

        # Connect to the target
        print(f"\nConnecting to the hidden service: {duckduckgo_addr}")
        response = py_arti.connect(
            duckduckgo_addr,
            443,
        )
        print(response)
            
    except Exception as e:
        print(e)
        return

if __name__ == "__main__":
    asyncio.run(hs_client_test())