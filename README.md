# cas-offinder-rust


Right now, there is full support for the functionality avaliable in cas-offinder version 2.5.

Note that bulges are not yet supported, however a script can be used to perform searches with bulges.

### Build:

First install rust and opencl on your system. Then:

```
cd cas-offinder-cli
cargo build --release
# the target binary should be in target/release/cas-offinder-cli
```

### Regression tests:

After building, you can run regression tests against downloaded versions of cas-offinder with: 

```
cd cas-offinder-cli
# download test data from UCSC database
python tests/download.py 
# compares results against download of v2.5 of cas-offinder
python tests/compare_impls.py tests/example0.in G
```