# Atha

## genome

```shell
mkdir -p ~/data/gams/Atha/genome
cd ~/data/gams/Atha/genome

# download
wget -N https://ftp.ensemblgenomes.ebi.ac.uk/pub/plants/release-58/fasta/arabidopsis_thaliana/dna/Arabidopsis_thaliana.TAIR10.dna_sm.toplevel.fa.gz

wget -N https://ftp.ensemblgenomes.ebi.ac.uk/pub/plants/release-58/gff3/arabidopsis_thaliana/Arabidopsis_thaliana.TAIR10.58.gff3.gz

# chromosomes
gzip -dcf *dna_sm.toplevel* |
    faops order stdin <(for chr in $(seq 1 1 5) Mt Pt; do echo $chr; done) stdout |
    pigz > genome.fa.gz
faops size genome.fa.gz > chr.sizes

# annotations
gzip -dcf Arabidopsis_thaliana.TAIR10.58.gff3.gz |
    grep -v '^#' |
    cut -f 1 |
    sort | uniq -c
# 213207 1
# 122450 2
# 152568 3
# 121665 4
# 180531 5
#    615 Mt
#    528 Pt

gzip -dcf Arabidopsis_thaliana.TAIR10.58.gff3.gz |
    grep -v '^#' |
    cut -f 3 |
    sort | uniq -c
# 286067 CDS
#      7 chromosome
# 313952 exon
#  56384 five_prime_UTR
#  27655 gene
#   3879 lnc_RNA
#    325 miRNA
#  48359 mRNA
#    377 ncRNA
#   5178 ncRNA_gene
#     15 rRNA
#    287 snoRNA
#     82 snRNA
#  48308 three_prime_UTR
#    689 tRNA

spanr gff Arabidopsis_thaliana.TAIR10.58.gff3.gz --tag CDS -o cds.json

faops masked genome.fa.gz |
    spanr cover stdin -o repeats.json

spanr merge repeats.json cds.json -o anno.json

spanr stat chr.sizes anno.json --all
#key,chrLength,size,coverage
#cds,119667750,33775569,0.2822
#repeats,119667750,38274794,0.3198

```

## T-DNA

```shell
mkdir -p ~/data/gams/Atha/features/
cd ~/data/gams/Atha/features/

for name in CSHL FLAG MX RATM; do
    aria2c -j 4 -x 4 -s 2 --file-allocation=none -c \
        http://natural.salk.edu/database/tdnaexpress/T-DNA.${name}
done

# Convert to ranges
for name in CSHL FLAG MX RATM; do
    cat T-DNA.${name} |
         perl -nla -e '
            @F >= 2 or next;
            next unless $F[1];

            my ( $chr, $pos ) = split /:/, $F[1];
            $chr =~ s/chr0?//i;
            $pos =~ s/^0+//;
            next unless $chr =~ /^\d+$/;

            print "$chr:$pos";
        ' \
        > T-DNA.${name}.rg;
done

```

## `gams`

### Contigs

```shell
cd ~/data/gams/Atha/

gams env --all
gams status stop

# start redis-server
redis-server &

gams status drop

gams gen genome/genome.fa.gz --piece 500000

gams status dump dumps/ctg.rdb

# tsv exports
time gams tsv -s 'ctg:*' |
    gams anno -H genome/cds.json stdin |
    gams anno -H genome/repeats.json stdin |
    rgr sort -H -f 2 stdin |
    pigz \
    > tsvs/ctg.tsv.gz
#real    0m1.153s
#user    0m1.224s
#sys     0m0.107s

gzip -dcf tsvs/ctg.tsv.gz |
    sed '1d' |
    cut -f 1 \
    > ctg.lst

# rg
time gams rg \
    features/T-DNA.CSHL.rg \
    features/T-DNA.FLAG.rg \
    features/T-DNA.MX.rg \
    features/T-DNA.RATM.rg
#real    0m7.715s
#user    0m0.371s
#sys     0m0.134s

time gams tsv -s 'rg:*' |
    gams anno genome/cds.json stdin -H |
    gams anno genome/repeats.json stdin -H |
    rgr sort -H -f 2 stdin |
    pigz \
    > tsvs/range.tsv.gz
#real    0m9.582s
#user    0m4.132s
#sys     0m3.299s

gams status dump dumps/rg.rdb

# stop the server
gams status stop

```

### Features and fsw

```shell
cd ~/data/gams/Atha/

gams status drop

gams gen genome/genome.fa.gz --piece 500000

parallel -j 4 -k --line-buffer '
    echo {}
    gams feature features/T-DNA.{}.rg --tag {}
    ' ::: CSHL FLAG MX RATM

time gams tsv -s 'feature:*' |
    gams anno genome/cds.json stdin -H |
    gams anno genome/repeats.json stdin -H |
    rgr sort -H -f 2 stdin |
    pigz \
    > tsvs/feature.tsv.gz
#real    0m8.533s
#user    0m3.722s
#sys     0m3.463s

gams status dump dumps/feature.rdb

# sw
time gams sw feature -a gc -p 8 |
    gams anno genome/cds.json stdin -H |
    gams anno genome/repeats.json stdin -H |
    rgr sort -H -f 2 stdin |
    pigz \
    > tsvs/fsw.tsv.gz
#real    0m54.430s
#user    2m20.268s
#sys     0m5.439s

```

### GC-wave

```shell
cd ~/data/gams/Atha/

gams status drop

gams gen genome/genome.fa.gz --piece 500000

time gams wave \
    --size 100 --step 10 --lag 100 \
    --threshold 3.0 --influence 1.0 \
    --coverage 0.2 -p 8 |
    rgr sort -H stdin \
    > tsvs/peak.tsv
#real    0m3.169s
#user    0m23.031s
#sys     0m0.367s

tsv-summarize tsvs/peak.tsv \
    -H --group-by signal --count
#signal  count
#1       26752
#-1      22651

# Loading peaks
time gams peak tsvs/peak.tsv
#real    0m4.809s
#user    0m1.183s
#sys     0m1.429s

gams tsv -s "peak:*" |
    rgr sort -f 2 -H stdin \
    > tsvs/wave.tsv

cat tsvs/wave.tsv |
    tsv-summarize -H --count
# 49148

cat tsvs/wave.tsv |
    tsv-filter -H --gt left_wave_length:0 |
    tsv-summarize -H --mean left_wave_length
#2296.94839883

cat tsvs/wave.tsv |
    tsv-filter -H --gt right_wave_length:0 |
    tsv-summarize -H --mean right_wave_length
#2279.6830973

tsv-filter tsvs/wave.tsv -H --or \
    --le left_wave_length:0 --le right_wave_length:0 |
    tsv-summarize -H --count
# 427

```

## clickhouse

* server

```shell
cd ~/data/gams/Atha/

mkdir -p clickhouse
cd clickhouse
clickhouse server

```

* load

```shell
cd ~/data/gams/Atha/

for q in ctg fsw; do
    clickhouse client --query "DROP TABLE IF EXISTS ${q}"
    clickhouse client --query "$(cat sqls/ddl/${q}.sql)"
done

for q in ctg fsw; do
    echo ${q}
    gzip -dcf tsvs/${q}.tsv.gz |
        clickhouse client --query "INSERT INTO ${q} FORMAT TSVWithNames"
done

```

* queries

```shell
cd ~/data/gams/Atha/

mkdir -p stats

# summary
ARRAY=(
    'ctg::length'
    'fsw::gc_content'
)

for item in "${ARRAY[@]}"; do
    echo ${item} 1>&2
    TABLE="${item%%::*}"
    COLUMN="${item##*::}"

    clickhouse client --query "$(
        cat sqls/summary.sql | sed "s/_TABLE_/${TABLE}/" | sed "s/_COLUMN_/${COLUMN}/"
    )"
done |
    tsv-uniq \
    > stats/summary.tsv

for t in fsw; do
    echo ${t} 1>&2
    clickhouse client --query "$(cat sqls/summary-type.sql | sed "s/_TABLE_/${t}/")"
done |
    tsv-uniq \
    > stats/summary-type.tsv

# fsw
for q in fsw-distance fsw-distance-tag; do
    echo ${q}
    clickhouse client --query "$(cat sqls/${q}.sql)" > stats/${q}.tsv
done

```

## plots

### fsw-distance-tag

```shell
cd ~/data/gams/Atha/

mkdir -p plots

cat stats/fsw-distance-tag.tsv |
    cut -f 1 |
    grep -v "^tag$" |
    tsv-uniq \
    > plots/tag.lst

for tag in $(cat plots/tag.lst); do
    echo ${tag}
    base="fsw-distance-tag.${tag}"

    cat stats/fsw-distance-tag.tsv |
        tsv-filter -H --str-eq tag:${tag} |
        tsv-select -H --exclude tag \
        > plots/${base}.tsv

    for y in {2..7}; do
        echo ${y}
        Rscript plot_xy.R --infile plots/${base}.tsv --ycol ${y} --yacc 0.002 --outfile plots/${base}.${y}.pdf
    done

    gs -q -dNOPAUSE -dBATCH -sDEVICE=pdfwrite -sOutputFile=plots/${base}.pdf \
        $( for y in {2..7}; do echo plots/${base}.${y}.pdf; done )

    for y in {2..7}; do
        rm plots/${base}.${y}.pdf
    done

    pdfjam plots/${base}.pdf --nup 7x1 --suffix nup -o plots

    pdfcrop --margins 5 plots/${base}-nup.pdf
    mv plots/${base}-nup-crop.pdf plots/${base}-nup.pdf

    rm plots/${base}.tsv
done

#gs -q -dNOPAUSE -dBATCH -sDEVICE=pdfwrite -sOutputFile=plots/fsw-distance-tag.pdf \
#    $( for tag in $(cat plots/tag.lst); do echo plots/fsw-distance-tag.${tag}-nup.pdf; done )
#
#pdfjam plots/fsw-distance-tag.pdf --nup 1x5 --suffix nup -o plots


```

