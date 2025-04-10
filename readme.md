# PyArti

This is a Python library that provides a convenient way to build custom circuits for Tor. You can build 3-hop circuits using custom relays, and then you can use the circuit to make HTTP requests to a target server.

## Prerequisites

Before using this library, you need to have the following installed:

- Python 3.8 or later
- Install the pyarti library using pip


## Usage

Here's an example of how to use the library:

```python
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
        py_arti.init()

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
```

## Sample Output of client_test method:

```
Creating firsthop circuit...
TCP connection established successfully to 88.198.35.49:443
Created the firsthop circuit.

Extending the circuit...
TCP connection established successfully to 5.2.68.154:443
Extended the circuit.

Extending the circuit...
TCP connection established successfully to 185.220.100.241:9000
Extended the circuit.

Connecting to the target...
HTTP/1.1 200 OK
Content-Type: text/html
ETag: "84238dfc8092e5d9c0dac8ef93371a07:1736799080.121134"
Last-Modified: Mon, 13 Jan 2025 20:11:20 GMT
Cache-Control: max-age=2143
Date: Fri, 21 Feb 2025 13:48:26 GMT
Content-Length: 1256
Connection: close
X-N: S

<!doctype html>
<html>
<head>
    <title>Example Domain</title>

    <meta charset="utf-8" />
    <meta http-equiv="Content-type" content="text/html; charset=utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <style type="text/css">
    body {
        background-color: #f0f0f2;
        margin: 0;
        padding: 0;
        font-family: -apple-system, system-ui, BlinkMacSystemFont, "Segoe UI", "Open Sans", "Helvetica Neue", Helvetica, Arial, sans-serif;

    }
    div {
        width: 600px;
        margin: 5em auto;
        padding: 2em;
        background-color: #fdfdff;
        border-radius: 0.5em;
        box-shadow: 2px 3px 7px 2px rgba(0,0,0,0.02);
    }
    a:link, a:visited {
        color: #38488f;
        text-decoration: none;
    }
    @media (max-width: 700px) {
        div {
            margin: 0 auto;
            width: auto;
        }
    }
    </style>
</head>

<body>
<div>
    <h1>Example Domain</h1>
    <p>This domain is for use in illustrative examples in documents. You may use this
    domain in literature without prior coordination or asking for permission.</p>
    <p><a href="https://www.iana.org/domains/example">More information...</a></p>
</div>
</body>
</html>
```

## Sample Output of hs_client_test method:

```
Connecting to the hidden service: duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion
Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Received 1024 bytes (total: 1024)
Received 1024 bytes (total: 2048)
Received 1024 bytes (total: 3072)
Received 1024 bytes (total: 4096)
Received 1024 bytes (total: 5120)
Received 1024 bytes (total: 6144)
Received 1024 bytes (total: 7168)
Received 1024 bytes (total: 8192)
Received 1024 bytes (total: 9216)
Received 1024 bytes (total: 10240)
Received 1024 bytes (total: 11264)
Received 1024 bytes (total: 12288)
Received 1024 bytes (total: 13312)
Received 1024 bytes (total: 14336)
Received 1024 bytes (total: 15360)
Received 1024 bytes (total: 16384)
Connecting through the custom circuit:
Relay 0: 193.11.164.243:9001
Relay 1: 38.152.218.16:443
Relay 2: 192.42.116.178:9000

Received 1024 bytes (total: 17408)
Received 1024 bytes (total: 18432)
Received 1024 bytes (total: 19456)
Received 1024 bytes (total: 20480)
Received 1024 bytes (total: 21504)
Received 1024 bytes (total: 22528)
Received 1024 bytes (total: 23552)
Received 1024 bytes (total: 24576)
Received 1024 bytes (total: 25600)
Received 1024 bytes (total: 26624)
Received 1024 bytes (total: 27648)
Received 1024 bytes (total: 28672)
Received 1024 bytes (total: 29696)
Received 1024 bytes (total: 30720)
Received 1024 bytes (total: 31744)
Received 1024 bytes (total: 32768)
Received 1024 bytes (total: 33792)
Received 1024 bytes (total: 34816)
Received 1024 bytes (total: 35840)
Received 1024 bytes (total: 36864)
Received 1024 bytes (total: 37888)
Received 1024 bytes (total: 38912)
Received 1024 bytes (total: 39936)
Received 1024 bytes (total: 40960)
Received 1024 bytes (total: 41984)
Received 1024 bytes (total: 43008)
Received 1024 bytes (total: 44032)
Received 1024 bytes (total: 45056)
Received 663 bytes (total: 45719)
Response (first 45719 bytes):
HTTP/1.1 200 OK
Server: nginx
Date: Wed, 19 Mar 2025 14:18:19 GMT
Content-Type: text/html; charset=UTF-8
Content-Length: 43361
Connection: close
Vary: Accept-Encoding
ETag: "67d9bf52-a961"
Strict-Transport-Security: max-age=0
Permissions-Policy: interest-cohort=()
Content-Security-Policy: default-src 'none' ; connect-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; manifest-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; media-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; script-src blob:  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com 'unsafe-inline' 'unsafe-eval' ; f
Total response size: 45719 bytes
HTTP/1.1 200 OK
Server: nginx
Date: Wed, 19 Mar 2025 14:18:19 GMT
Content-Type: text/html; charset=UTF-8
Content-Length: 43361
Connection: close
Vary: Accept-Encoding
ETag: "67d9bf52-a961"
Strict-Transport-Security: max-age=0
Permissions-Policy: interest-cohort=()
Content-Security-Policy: default-src 'none' ; connect-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; manifest-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; media-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; script-src blob:  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com 'unsafe-inline' 'unsafe-eval' ; f
```

## Sample Output of hs_client_test (load directory from cache)

### first run
```
load directory from cache
bootstrap manually

Connecting to the hidden service: duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion
HTTP/1.1 200 OK
Server: nginx
Date: Tue, 01 Apr 2025 09:12:11 GMT
Content-Type: text/html; charset=UTF-8
Content-Length: 45072
Connection: close
Vary: Accept-Encoding
ETag: "67eb0842-b010"
Strict-Transport-Security: max-age=0
Permissions-Policy: interest-cohort=()
Content-Security-Policy: default-src 'none' ; connect-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; manifest-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; media-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; script-src blob:  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com 'unsafe-inline' 'unsafe-eval' ; f
```
### second run
```
load directory from cache

Connecting to the hidden service: duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion
HTTP/1.1 200 OK
Server: nginx
Date: Tue, 01 Apr 2025 09:13:12 GMT
Content-Type: text/html; charset=UTF-8
Content-Length: 45072
Connection: close
Vary: Accept-Encoding
ETag: "67eb0842-b010"
Strict-Transport-Security: max-age=0
Permissions-Policy: interest-cohort=()
Content-Security-Policy: default-src 'none' ; connect-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; manifest-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; media-src  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com ; script-src blob:  https://duckduckgo.com https://*.duckduckgo.com https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/ https://spreadprivacy.com 'unsafe-inline' 'unsafe-eval' ; f
```
