#!/bin/bash
# A script to set up a local MMO development environment in tmux

SESSION_NAME="mmo-server"

tmux rename-window 'MMO Dev'
tmux send-keys -t $SESSION_NAME:2 "cd web-server && cargo run" C-m

tmux split-window -v -t $SESSION_NAME:2 
tmux send-keys -t $SESSION_NAME:2.1 "cd mmo-server && cargo run --features agones" C-m 

tmux split-window -h -t $SESSION_NAME:2.0 
tmux send-keys -t $SESSION_NAME:2.2 "agones-server --local" C-m

