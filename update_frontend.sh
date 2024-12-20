#!/bin/bash

cd frontend

DEST_PATH="../server/build"

# Check if the destination directory exists, if not create it
if [ ! -d "$DEST_PATH" ]; then
    mkdir -p "$DEST_PATH"
fi

npm run build

rm -r $DEST_PATH/*
cp -r dist/index.html $DEST_PATH/
cp -r dist/assets $DEST_PATH

cd ..

echo "Frontend build updated successfully!"
