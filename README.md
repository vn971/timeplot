## About

TimePlot -- personal activity tracker & visualizer.

Visualizing your performance can help you understand how certain things impact your computer work, properly bill customers for freelance tasks or potentially fight procrastination.
Or maybe just get new cool graphs.;D


## Usage

* Run "timeplot"
* Each 3 minutes your currently active window name is logged to `~/.local/share/timeplot/log.log`. Open the log to see if timeplot has categorized your activity correctly. It looks like this:
```
2018-10-01_14:00 skip 0 Desktop
2018-10-01_15:03 work 9 #rust @ irc.mozilla.org
2018-10-01_19:11 fun 18 The Battle for Wesnoth
2018-10-01_20:38 skip 0 Desktop
2018-10-01_21:31 personal 13 vasya@vn971think:~
```
* If the category is wrong, fix the category right in the log.
* Edit rules to auto-categorize this window name in the future: `~/.config/timeplot/rules_simple.txt`.
* Wait for timeplot to re-draw the plot (`~/.cache/timeplot/svg.svg`)
<img src="docs/png.png" width="800" /><!-- screenshot params: pngcairo 1200,170, 2.9 -->
* Whenever you want to check the text log, or see if it can be improved, return 3 steps back.


## Hints

* You can set this image as your Desktop background image if you like.
* You can configure the app by editing ~/.config/timeplot/config.toml  (plot a different number of days, configure colors, statistics display, etc).
* If you have trouble finding the directories, run `timeplot` from terminal. It will print the directories.
* If you're curious on what the number means in the logs: it means your desktop "workstation" number, usually 1-4. It's logged, but it's not yet usable in "rules_simple.txt". Hopefully it'll be usable in future versions of timeplot.


## Installation (ArchLinux)

TODO: AUR package


## Installation (Linux, simple)

* Install dependencies: `gnuplot` `xprintidle` `xdotool`
* Download the binary from https:// TODO
* Make it executable (for example, `chmod +x timeplot`)
* Run it


## Installation (MacOS, Linux)

* Install dependencies: `gnuplot` `xprintidle` `xdotool`
* Install `cargo` (the Rust build tool)
* Download this project-s sources, enter the directory and build: `cargo build --release`
* Run `target/release/timeplot`


## Installation (Windows)

* Not supported yet. We need to implement window name extraction, then it should work.


## Other

The application only does what it says in this description. It never sends anything anywhere, never logs any kind of data except the one specified above.

The app is shared under GPLv3+. Sources can be found here [https://github.com/vn971/timeplot](https://github.com/vn971/timeplot) and here [https://gitlab.com/vn971/timeplot](https://gitlab.com/vn971/timeplot)
