import subprocess
from pprint import pprint
import difflib
import tempfile
import sys
import os
import shutil
import random

def line_cmp(line):
    rna,chrom, loc, dna, dir, mism = line.split(b'\t')[:6]
    return (chrom, int(loc), dir, rna, mism)

def sort_data(bdata):
    lines = bdata.splitlines(keepends=True)
    lines.sort(key=line_cmp)
    return b"".join(lines)

def get_gold_data(in_filename, device):
    data_fname = in_filename+".gold"
    if not os.path.exists(data_fname):
        print("Generating gold comparison data")
        args = [in_filename, 'G', data_fname]
        subprocess.run(["./bin/cas-offinder-2.exe"] + args,stdout=subprocess.PIPE)
    with open(data_fname,'rb') as f:
        return f.read()

def compare_on_input(input_filename, device):
    args = [input_filename, device, '-']
    print("started",flush=True)
    out2 = get_gold_data(input_filename, device)
    print("finishedgold",flush=True)
    out1 = subprocess.run(["./target/release/cas-offinder-cli.exe"] + args,stdout=subprocess.PIPE).stdout
    print("finished test")

    with tempfile.NamedTemporaryFile() as file1, \
       tempfile.NamedTemporaryFile() as file2:
        file1.write(sort_data(out1))
        file1.flush()
        file2.write(sort_data(out2))
        file2.flush()
        open(input_filename+".gold.sorted",'wb').write(sort_data(out2))
        open(input_filename+".test.sorted",'wb').write(sort_data(out1))
        out = subprocess.run(["diff",file1.name, file2.name])
        if out.returncode == 0:
            print("files are the same")
        else:
            print("files differed")     


if __name__ == "__main__":
    assert len(sys.argv) == 3, "requires 2 arguments: input file, device"
    compare_on_input(sys.argv[1], sys.argv[2])
