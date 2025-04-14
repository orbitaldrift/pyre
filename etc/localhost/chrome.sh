#!/bin/bash

echo "Opening google chrome"

case `uname` in
    (*Linux*)  google-chrome --origin-to-force-quic-on=127.0.0.1:4433 --enable-logging --v=1 ;;
    (*Darwin*)  open -a "Google Chrome" --args --origin-to-force-quic-on=127.0.0.1:4433 --enable-logging --v=1 ;;
esac

## Logs are stored to ~/Library/Application Support/Google/Chrome/chrome_debug.log