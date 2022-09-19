# cas-offinder-rust


Right now, there is full support for the functionality avaliable in cas-offinder version 2.5.

Note that bulges are not yet supported, however a script can be used to perform searches with bulges.

### Build

First install rust and opencl on your system. Then:

```
cd cas-offinder-cli
cargo build --release
# the target binary should be in target/release/cas-offinder-cli
```

### Regression tests

After building, you can run regression tests against downloaded versions of cas-offinder with: 

```
cd cas-offinder-cli
# download test data from UCSC database (will not fetch again if already fetched)
python tests/download.py 
# compares results against download of v2.5 of cas-offinder
python tests/compare_impls.py tests/example0.in G
```

### Benchmarking

There is also a benchmarking tool to compare this implementation's performance against the original cas-offinder on a wide variety of input sizes and device types. To run it (after building):

```
cd cas-offinder-cli
# download test data from UCSC database (will not fetch again if already fetched)
python tests/download.py 
# compares results against download of v2.5 of cas-offinder vs
python tests/benchmark.py
```

#### Benchmarking results

I ran this on my new desktop (CPU: Ryzen 3900x, GPU: 2x RTX 3060), and got the following results:

```
Device | Num Guides | Mismatches | PAM | Guide Length | new code |  cas-offinder 2.4.1
--- | --- | --- | --- | --- | --- | ---
C | 1 | 3 | NRG | 20 | 18.03459334373474 | **0.8828732967376709**
C | 1 | 3 | NRG | 25 | 17.84967803955078 | **0.8931279182434082**
C | 1 | 3 | NNGRRT | 20 | 6.3219215869903564 | **0.8964118957519531**
C | 1 | 3 | NNGRRT | 25 | 5.67253851890564 | **0.8786423206329346**
C | 1 | 3 | TTTN | 20 | 8.489312887191772 | **0.891948938369751**
C | 1 | 3 | TTTN | 25 | 8.10775899887085 | **0.8844027519226074**
C | 1 | 6 | NRG | 20 | 19.04048776626587 | **0.9391365051269531**
C | 1 | 6 | NRG | 25 | 19.065585374832153 | **1.106593370437622**
C | 1 | 6 | NNGRRT | 20 | 6.491907835006714 | **0.8924095630645752**
C | 1 | 6 | NNGRRT | 25 | 5.715454816818237 | **0.8866109848022461**
C | 1 | 6 | TTTN | 20 | 8.655573844909668 | **0.8951475620269775**
C | 1 | 6 | TTTN | 25 | 8.461794376373291 | **0.8994026184082031**
C | 100 | 3 | NRG | 20 | 126.5570867061615 | **45.08907914161682**
C | 100 | 3 | NRG | 25 | 128.03703713417053 | **44.70433783531189**
C | 100 | 3 | NNGRRT | 20 | **21.781389713287354** | 44.895357847213745
C | 100 | 3 | NNGRRT | 25 | **20.746649742126465** | 44.605469703674316
C | 100 | 3 | TTTN | 20 | **41.97769284248352** | 44.82026433944702
C | 100 | 3 | TTTN | 25 | **44.052568197250366** | 44.73158001899719
C | 100 | 6 | NRG | 20 | 229.0389130115509 | **45.28039765357971**
C | 100 | 6 | NRG | 25 | 226.2367491722107 | **45.128363847732544**
C | 100 | 6 | NNGRRT | 20 | **34.68102216720581** | 45.236823081970215
C | 100 | 6 | NNGRRT | 25 | **32.937326192855835** | 45.06292200088501
C | 100 | 6 | TTTN | 20 | 73.12770247459412 | **45.373504877090454**
C | 100 | 6 | TTTN | 25 | 75.0310845375061 | **45.21669292449951**
G | 1 | 3 | NRG | 20 | 4.1110520362854 | **0.7049260139465332**
G | 1 | 3 | NRG | 25 | 4.8200156688690186 | **0.7068839073181152**
G | 1 | 3 | NNGRRT | 20 | 3.41017484664917 | **0.8476431369781494**
G | 1 | 3 | NNGRRT | 25 | 3.549423933029175 | **0.6853504180908203**
G | 1 | 3 | TTTN | 20 | 3.572619676589966 | **0.7564003467559814**
G | 1 | 3 | TTTN | 25 | 3.6441240310668945 | **0.69614577293396**
G | 1 | 6 | NRG | 20 | 4.280444860458374 | **0.8860418796539307**
G | 1 | 6 | NRG | 25 | 5.3128578662872314 | **0.6940240859985352**
G | 1 | 6 | NNGRRT | 20 | 3.4332568645477295 | **0.6886427402496338**
G | 1 | 6 | NNGRRT | 25 | 3.549546718597412 | **0.8710250854492188**
G | 1 | 6 | TTTN | 20 | 3.6664655208587646 | **0.8714864253997803**
G | 1 | 6 | TTTN | 25 | 3.7021522521972656 | **0.682518482208252**
G | 100 | 3 | NRG | 20 | 59.699851274490356 | **2.9909822940826416**
G | 100 | 3 | NRG | 25 | 134.9799792766571 | **3.79887318611145**
G | 100 | 3 | NNGRRT | 20 | 12.573736190795898 | **3.704956293106079**
G | 100 | 3 | NNGRRT | 25 | 5.552325487136841 | **3.179410696029663**
G | 100 | 3 | TTTN | 20 | 31.327723264694214 | **2.9365100860595703**
G | 100 | 3 | TTTN | 25 | 25.85095191001892 | **3.7123799324035645**
G | 100 | 6 | NRG | 20 | 86.17052340507507 | **11.165757894515991**
G | 100 | 6 | NRG | 25 | 184.31043076515198 | **3.4571826457977295**
G | 100 | 6 | NNGRRT | 20 | 16.984584093093872 | **11.999996423721313**
G | 100 | 6 | NNGRRT | 25 | 6.44679594039917 | **3.7016706466674805**
G | 100 | 6 | TTTN | 20 | 42.546626329422 | **15.598616361618042**
G | 100 | 6 | TTTN | 25 | 34.044556856155396 | **3.716431140899658**
```

As you can see, the GPU results are uniformly faster, often by a wide margin. The CPU results are mixed, but the worst case for the new implementation is only 2x slower than the old implementation. 