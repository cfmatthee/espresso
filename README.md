# espresso

A windows tray application that prevents the computer from sleeping, unless:

- the user manually locks the desktop, or
- the app is paused in the menu item

## Operating principle

This application uses the Windows _SetThreadExecutionState_ to prevent the app from sleeping.

There is no version other than the one design for operating on a Windows system. The functionality was only tested on a Windows 11 system.

## Similarity to existing applications

There are many applications that performs this tasks. I used [Caffeine](https://www.zhornsoftware.co.uk/caffeine/) from _Zhorn Software_ before starting to code my own.

## Disclaimer

This application is almost entirely _vibe coded_. It makes multiple system calls into the Windows system. Make sure you are comfortable with the calls made in the source code before using this application.
