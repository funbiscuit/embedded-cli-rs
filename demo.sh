#!/bin/bash
# This script emulates predefined input to showcase usage of CLI
# Dependencies: xdotool
# To run demo you need to flash arduino example to real arduino nano
# Then run script in background:
# ./demo.sh &
#
# To convert recorded mp4 it's best to use gifski:
# ffmpeg -i embedded-cli.mp4 frame%04d.png
# gifski -o demo.gif -Q 50 frame*.png

type () {
   xdotool type --delay 300 -- "$1"
}
submit () {
   xdotool key --delay 500 Return
}
backspace () {
   local repeat=${1:-1}
   xdotool key --delay 400 --repeat $repeat BackSpace
}
tab () {
   xdotool key --delay 500 Tab
}
left () {
   local repeat=${1:-1}
   xdotool key --delay 400 --repeat $repeat Left
}
right () {
   local repeat=${1:-1}
   xdotool key --delay 400 --repeat $repeat Right
}
up () {
   local repeat=${1:-1}
   xdotool key --delay 800 --repeat $repeat Up
}
down () {
   local repeat=${1:-1}
   xdotool key --delay 800 --repeat $repeat Down
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

type "--hlp"
left 2
type "e"
submit

sleep 0.5

up 2

type "Rust"
submit

type "got-l"
left 5
type "help "
right 2
backspace
type "e"
right 3
type "ed"
submit

type "g"
tab

type "-"
tab

type "12"
submit

up 2
down
backspace
type "3"
submit

type "test"
submit

up
type " 123 789"
backspace 3
type "456"
left 4
backspace 2
type "01"
submit

type "test \"101 456\""
submit

up
left 4
backspace 4
type "\\\"abc\\\" "
submit

# Wait until keys disappear
sleep 5
echo "Demo is finished"
