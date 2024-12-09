#!/bin/bash

cd frontend

DEST_PATH="../crate_midiserv/static"

# Check if the destination directory exists, if not create it
if [ ! -d "$DEST_PATH" ]; then
    mkdir -p "$DEST_PATH"
fi

npm run build

cp -r dist/index.html "$DEST_PATH"
cp -r dist/assets ../crate_midiserv

cd ..

echo "Frontend build updated successfully!"
