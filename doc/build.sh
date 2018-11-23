#!/bin/bash

# build book/apidoc for each tag
for tag in $(git tag)
do
    git checkout $tag
    mdbook build -d book/$tag
    cargo doc --target-dir book/api/$tag
done

# build book/apidoc for master
mdbook build -d book/master
cargo doc --target-dir book/api/master
