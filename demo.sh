#!/bin/bash
# This script emulates predefined input to showcase usage of CLI
# Dependencies: xdotool
# To run demo you need to flash arduino example to real arduino nano
# Then run script in background:
# ./demo.sh &
#
# To convert recorded mp4 it's best to use gifski:
# ffmpeg -i embedded-cli.mp4 frame%04d.png
# gifski -o demo.gif -Q 20 frame*.png

type () {
   xdotool type --delay 300 -- "$1"
}
submit () {
   xdotool key --delay 500 Return
}
backspace () {
   xdotool key --delay 500 BackSpace
}
tab () {
   xdotool key --delay 500 Tab
}
up () {
   xdotool key --delay 800 Up
}
down () {
   xdotool key --delay 800 Down
}

echo "Demo started"

# Connect to device
sleep 1
xdotool key ctrl+l
# For quick testing locally
#xdotool type "cargo run"
xdotool type "tio /dev/ttyUSB0 --map ODELBS"
xdotool key Return
# long sleep so initial keys disappear and arduino boots
sleep 5

type "help"
submit

type "h"
tab

sleep 0.5

type "l"
tab

sleep 0.5
submit

up

type "--help"
submit

sleep 0.5

up
up

type "Rust"
submit

type "help get-led"
submit

type "g"
tab

type "-"
tab

type "12"
submit

up
up
down
backspace
type "3"
submit

type "test"
submit

up
type " 123 789"
backspace
backspace
backspace
type "456"
submit

# Wait until keys disappear
sleep 5
echo "Demo is finished"
