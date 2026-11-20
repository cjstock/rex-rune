# Rex Rune

A native client for Rex terminal servers. These terms cover how the client's look is chosen and shared with servers.

## Appearance

**Chrome palette**:
The colours of the client's own interface (toolbar, sidebar, dialogs, pane headers), never of terminal content.
_Avoid_: UI theme, app theme

**Terminal theme**:
The default foreground, background, cursor and 256-colour palette this client declares to a server for the panes it owns.
_Avoid_: colour scheme, terminal colours

**Theme source**:
A file on this machine, written by something outside the client (DMS, matugen, by hand), that supplies both a chrome palette and a terminal theme, and that the client follows as it changes.
_Avoid_: system theme, wallpaper theme, DMS theme

**Owned pane**:
A pane whose block this client owns on its server, or that no client owns; the only panes a declared terminal theme reaches.
_Avoid_: attached pane, local pane

**Theme source template**:
The matugen template that ships with the client and makes DMS write the theme source every time it regenerates colours. The client installs it only when the user asks.
_Avoid_: DMS theme, matugen config
