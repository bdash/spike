# Rust interface to Spike 2 hardware on Stern pinball machines

This library interfaces with the Spike 2 hardware used in recent [Stern pinball
machines](https://www.sternpinball.com/). It is intended to be used by custom
software running on the pinball machine itself to control the pinball hardware.
It cannot interact with the pinball hardware from a different machine.

The included demo application provides a virtual keyboard driver that
translates switch events into keyboard events. You can see it in action being
used to play Doom II here:

[![Doom II running on Stern Avengers Infinity
Quest](https://img.youtube.com/vi/Nf8uIzg_aUA/0.jpg)](https://www.youtube.com/watch?v=Nf8uIzg_aUA).

## How do I use this?

Briefly:

1. Cross-compile for `armv7-unknown-linux-musleabihf`
2. Copy your app onto the game's storage
3. Run it.

If you want to use the virtual keyboard driver in the demo application, you'll
also need to build a [uinput kernel
module](https://kernel.org/doc/html/v4.12/input/uinput.html) for the Linux
version running on the Spike 2 CPU. This is needed to synthesize virtual
keyboard events.

I'm not going to provide more detailed instructions at this time because I
don't want to be responsible for you damaging your expensive pinball machine.

If you have it working and have specific questions, [feel free to email
me](mailto:mrowe@bdash.net.nz).

## Limitations

This currently only handles node bus initialization and processing switch
inputs. That was all that was needed for Doom.

Supporting solenoids and lamps would require adding support for a handful of
additional node bus messages. I didn't mess with them as I didn't have any
reason to and I wanted to minimize the risk of me doing a dumb.

## Thanks

Mission Pinball Framework's [MPF to
Spike bridge](https://github.com/missionpinball/mpf-spike/) provided good clues
for understanding the message numbers and structure of the node bus protocol.

