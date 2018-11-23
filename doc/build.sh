#!/bin/bash

# build book for each tag
for tag in $(git tag)
do
    mdbook build -d book/$tag
done

# build book for master
mdbook build -d book/master
