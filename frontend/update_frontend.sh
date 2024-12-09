#!/bin/bash

DEST_PATH="../crate_midiserv/static/"

# Check if the destination directory exists, if not create it
if [ ! -d "$DEST_PATH" ]; then
    mkdir -p "$DEST_PATH"
fi

# Build the frontend using npm
npm run build

# Copy the new build files to the destination directory
cp -r dist/index.html "$DEST_PATH"
cp -r dist/assets ../crate_midiserv

echo "Frontend build updated successfully!"
