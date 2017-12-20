#!/bin/bash

SELF=$0

sleeper() {
	local t=$1
	echo "[self=$$; ppid=${PPID}; t=${t}]"
	shift

	if [ "$#" = '0' ]; then
		exec sleep "$t"
	else
		$SELF "$@" &
		exec sleep "$t"
	fi
}

sleeper "$@"
