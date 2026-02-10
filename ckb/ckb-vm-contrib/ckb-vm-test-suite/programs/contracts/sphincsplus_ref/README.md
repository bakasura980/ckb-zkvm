# sphincsplus

Performance test of <https://github.com/sphincs/sphincsplus> on ckb-vm.

```sh
$ ln -s ../../scripts .
$ make SPHINCSPLUS_PARAMS=sphincs-shake-128f SPHINCSPLUS_THASH=simple
$ ckb-debugger --bin sphincsplus_ref
```

- All possible `SPHINCSPLUS_PARAMS` combinations `sphincs-[haraka/shake/sha2]-[128/192/256][f/s]`.
- All possible `SPHINCSPLUS_THASH` combinations `[simple/robust]`.

Use the following command to quickly test all combinations:

```sh
#!/usr/bin/env bash
set -e

thashes=("simple" "robust")
fns=("haraka" "shake" "sha2")
sizes=("128" "192" "256")
options=("f" "s")

for thash in "${thashes[@]}"; do
  for fn in "${fns[@]}"; do
    for size in "${sizes[@]}"; do
      for option in "${options[@]}"; do
        param="sphincs-${fn}-${size}${option}"
        echo "Building with $param and $thash..."
        make SPHINCSPLUS_PARAMS="$param" SPHINCSPLUS_THASH="$thash"
        ckb-debugger --bin sphincsplus_ref
      done
    done
  done
done
```
