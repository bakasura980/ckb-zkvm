# CKB VM Test Suite

Test suite for CKB VM, kept in a separate project to avoid polluting the vm repo with submodules

# How to run this

First, make sure you have clang-19 installed. To test this, make sure the following command works:

```bash
$ clang-19 --version
```

Now you can run the test suite with the following steps:

```bash
$ make ckb-vm
$ ./test.sh
```

# Get Coverage Report By Kcov

First install kcov by <https://github.com/SimonKagstrom/kcov/blob/master/INSTALL.md>

```sh
$ ./test.sh --coverage
```

# Benchmark

Get a full performance review of ckb-vm:

```sh
$ make report-bench.txt
```
