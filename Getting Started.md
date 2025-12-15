# Getting Started

Tapestry Loom consists of three types of views: The file manager, (application-wide) settings, and editors. Unlike the other two views, editors contain multiple subviews and you can have an arbitrary number of editors open.

Editors can either be temporary, or they can be backed by a file on disk (Tapestry Loom's document format has the file extension `.tapestry`). When an editor is backed by a file on disk, changes will be automatically saved to disk. This way, if the application crashes, only very recent changes will be lost.

## Adding a model in Settings

Before we can begin using Tapestry Loom, we'll need to add our first model.

Open the Settings view and scroll down to the Inference section. Click on the "Choose template..." dropdown and choose the inference template corresponding to your inference endpoint.

Fill out the relevant fields in the template and then click "Add model" to add the model to the model list.

Once the model has been added, you can modify it further. Adding a custom label and label color is usually a good idea.

Finally, make sure that everything looks correct (and correct any errors that you find) before proceeding.

### Setting your model as default

TODO

## Editor basics

Documents in Tapestry Loom are called weaves. In this section, we'll use the model we added earlier to write our first weave and then save it to disk.

## Managing weaves

## Advanced editor usage

In this section, we'll explore the power user features of Tapestry Loom.

<!--
Stuff to bring up:
- Alternate views
- Cursors and hovered nodes
- Automatic scrolling
- Keyboard shortcuts
- Shift-click and context menus
- Hover information
- Menu and metadata
	- Logprob nodees
-->