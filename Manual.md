# Tapestry Loom Manual

Tapestry Loom consists of three types of views: The file manager, (application-wide) settings, and editors. Unlike the other two views, editors contain multiple subviews and you can have an arbitrary number of editors open.

Editors can either be temporary, or they can be backed by a file on disk (Tapestry Loom's document format has the file extension `.tapestry`). When an editor is backed by a file on disk, changes will be automatically saved to disk. This way, if the application crashes, only very recent changes will be lost.

Tapestry Loom documents are called weaves, and will be referred to as such throughout the rest of this document.

## Settings view

The settings view is split into multiple sections:

- Interface
- Shortcuts
- Document
- Inference
- Editor inference defaults

The exact settings available will change over time; This document will only provide a brief overview of the most important available settings.

More information about a specific setting can often be found by hovering over the setting's UI element until a tooltip appears.

### Shortcuts

Tapestry Loom has various keyboards shortcuts which can allow the user to more quickly execute common actions. You can click on a shortcut's button to change it, and you can press escape while editing a shortcut to clear the keybind.

On MacOS, shortcuts containing Ctrl can be triggered using either the control or command keyboard buttons.

### Document

The root location specifies the path used by the built-in file manager. File paths in the UI are abbreviated to be relative to the root location whenever possible.

Documents are automatically saved at fixed intervals based on the configured autosave interval. In addition, the app will automatically save all open documents before exiting.

### Inference

This section allows you to manage the models used for generating new nodes. New models can be added to the model list by filling out one of the built-in templates.

Models in the list have a label, an optional color (used for color coding their outputs in the editor), and various API-specific options. The list can be in any order, and models can be moved up and down the list.

Once a model is added to the list, the API it uses cannot be changed without deleting and re-adding the model.

### Inference defaults

This section allows you to change the default inference parameters used in new editors. See the [editor menu subview](#menu-subview) for more information on how to use this.

### Inference presets

## File manager view

## Editor view

### Tree subview

### \[Text\] Editor subview

### Bookmarks subview

### Menu subview

### Graph subview

### List subview

### Canvas subview