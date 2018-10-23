## TimePlot  [![Build Status](https://travis-ci.org/vn971/timeplot.svg?branch=master)](https://travis-ci.org/vn971/timeplot)  [![crates.io](https://img.shields.io/crates/v/timeplot.svg)](https://crates.io/crates/timeplot)

Log your activity, visualize and analyze it.

Visualizing your performance can help you understand how certain things impact your computer work, properly bill customers for freelance tasks and potentially fight procrastination. Or maybe just get new cool graphs.:)


## Usage

* Run "timeplot"
* Each 3 minutes your currently active window name is logged. Open the log to see if timeplot has categorized your activity correctly. It looks like this:
```
2018-10-01_14:00 skip 0 Desktop
2018-10-01_15:03 work 9 #rust @ irc.mozilla.org
2018-10-01_19:11 fun 18 The Battle for Wesnoth
2018-10-01_20:38 skip 0 Desktop
2018-10-01_21:31 personal 13 vasya@vn971think:~
```
* If the category is wrong, fix the category right in the log.
* Edit rules to auto-categorize this window name in the future
* Wait for timeplot to re-draw the image
<img src="docs/png.png" width="800" /><!-- screenshot params: pngcairo 1200,170, 2.9 -->
* Whenever you want to check the text log, or see if it can be improved, return 3 steps back.


## Hints

* You can set the image as your Desktop background image if you like.
* You can configure the app:
* * plot a different number of days
* * colors, statistics display
* * run configured subcommands whenever a particular category is encountered


## Installation

1. Make sure dependencies are installed:
* * On Debian/Ubuntu, `sudo apt install gnuplot xprintidle xdotool`
* * On ArchLinux, `pacman -S --needed gnuplot xprintidle xdotool`
* * On Windows, install [gnuplot](https://sourceforge.net/projects/gnuplot/files/gnuplot/)
* * On macOS, `brew install gnuplot`
2. Build the project:
* * On all platforms, if you're familiar with Rust+cargo, install via `cargo install timeplot`.
* * On Linux, you can download pre-built version: [https://pointsgame.net/vn971/temp/tpl/timeplot](https://pointsgame.net/vn971/temp/tpl/timeplot)  and make it executable by doing `chmod +x timeplot`
* * On all platforms, clone/download this repository, install `cargo`, build project with `cargo build --release`, observe the executable on "target/release/timeplot".
3. Consider adding `timeplot` to autostart, making it run when you log in. If you use macOS or Windows, you must create said autostart hook manually (help on allowing to automate it appreciated). For Linux, there's a configuration setting that, if enabled, will create XDG autostart entry for you.


## Other

The application only does what it says in this description. It never sends anything anywhere, never logs any kind of data except the one specified above.

The app is shared under GPLv3+. Sources can be found here [https://github.com/vn971/timeplot](https://github.com/vn971/timeplot) and here [https://gitlab.com/vn971/timeplot](https://gitlab.com/vn971/timeplot)
