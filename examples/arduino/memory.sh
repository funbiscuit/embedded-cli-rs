#!/bin/bash
# This script measures ROM and static RAM memory usage of Arduino example with different features enabled
# Memory info should be updated at following places:
# /README.md in Demo section
# /README.md in Memory section
# /examples/arduino/README.md in example description

memory_file="target/MEMORY.md"

echo "## Memory usage" >$memory_file

echo "| Features | ROM, bytes | Static RAM, bytes |" >>$memory_file
echo "|----------|:----------:|:-----------------:|" >>$memory_file

array=("autocomplete" "history" "help")
n=${#array[@]}
for ((i = 0; i < (1 << n); i++)); do
   list=()
   list_md=()
   for ((j = 0; j < n; j++)); do
      if (((1 << j) & i)); then
         list+=("${array[j]}")
         list_md+=("\`${array[j]}\`")
      fi
   done
   features=$(
      IFS=,
      echo "${list[*]}"
   )
   features_md=$(
      IFS=' '
      echo "${list_md[*]}"
   )
   echo "Measuring features: $features"

   cp Cargo.toml Cargo.toml.bak

   cargo remove embedded-cli
   cargo add embedded-cli --path "../../embedded-cli" \
      --no-default-features \
      --features "macros, $features"

   cargo build --release

   ram_usage=$(avr-nm -Crtd --size-sort \
      target/avr-atmega328p/release/arduino-cli.elf |
      grep -i ' [dbvr] ' |
      awk -F " " '{Total=Total+$1} END{print Total}' -)

   rom_usage=$(cargo bloat --release --message-format json | jq -cs '.[0]["text-section-size"]')

   echo "| $features_md | $rom_usage | $ram_usage |" >>$memory_file
   echo "$features: ROM=$rom_usage RAM=$ram_usage"

   mv Cargo.toml.bak Cargo.toml
done
