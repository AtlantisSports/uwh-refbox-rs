#!/usr/bin/env bash

cargo install $1

R=$?

if [[ R -eq 0 ]] || [[ R -eq 101 ]]; then 
  exit 0
fi
  
exit ${R}
