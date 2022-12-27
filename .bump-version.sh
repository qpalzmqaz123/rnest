#!/bin/sh

VERSION=$1

if [ -z $VERSION ]; then
    echo "Usage $0 X.X.X.[(alpha|beta|rc)-X]" >&2
    exit 1
fi

echo "Bump version $VERSION"

FILES=`git ls-files | grep 'Cargo\.toml'`
for file in $FILES; do
    echo "Modify version field in file '$file'"
    sed -i "0,/version = \".*\"/s//version = \"$VERSION\"/" $file
done

# Modify version in Cargo.lock
cargo check
