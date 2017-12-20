#!/bin/bash

trap 'echo signal shatal && sleep 100' TERM

"$@"

