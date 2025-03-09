# This file is just an entry point for uvicorn
# The actual application is defined in zerg.main

if __name__ == "__main__":
    import uvicorn

    uvicorn.run("main:app", host="0.0.0.0", port=8001, reload=True)
