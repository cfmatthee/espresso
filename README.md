# espresso

A windows tray application that prevents the computer from sleeping, unless:

- the app is paused in the menu item,
- the user manually locks the desktop,
- the user manually suspends the computer (put it to sleep), or
- power saving settings forces a suspend (i.e. due to low battery).

## Operating principle

This application uses the Windows _SetThreadExecutionState_ to prevent the app from sleeping.

## Disclaimer

- This application is almost entirely _vibe coded_. Use with care.
- It makes multiple system calls into the Windows system. Make sure you are comfortable with the calls made in the source code before using this application.
- There is no version other than this one designed for operating on a Windows system.
- The functionality was only tested on a Windows 11 system.
- The icons are only design for a dark background.

## Similarity to existing applications

There are many applications that performs this tasks. I used [Caffeine](https://www.zhornsoftware.co.uk/caffeine/) from _Zhorn Software_ before starting to code my own.

## License

This is free and unencumbered software released into the public domain. For more information, please refer to the [LISENCE.txt](LISENCE.txt) file and to <https://unlicense.org/>.
