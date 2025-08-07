#!/bin/bash

# the -c-1 sets the cache timout to -1. meaning no cache.
# good for a dev server with hot reloading
http-server . -c-1 -p 8080 
