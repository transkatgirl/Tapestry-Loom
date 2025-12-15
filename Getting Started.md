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

Scroll down in the Settings view until you find the "Editor inference defaults" section. This setting allows you to change the default inference parameters used in new editors.

You can add your model to the inference parameters by clicking the "Choose model..." dropdown and selecting the model you added earlier. Once the model has been added, adjust the inference parameters based on your personal preferences.

## Editor basics

Documents in Tapestry Loom are called weaves. In this section, we'll use the model we added earlier to write our first weave and then save it to disk.

### Adjusting inference parameters

Before we start on our weave, let's make sure our inference parameters are correct.

The weave editor is split into multiple subviews, with inference parameters being located in the Menu subview.

Once the Menu subview is open, review the inference parameters being used and adjust them based on your personal preferences. Keep in mind that you can use multiple models for inference at the same time, but you need to make sure that you have at least one model selected.

Keep in mind that this subview's state is not persistent; Every time you open a weave, these parameters will initially be set to the defaults specified in settings.

Now that our inference parameters are correct, let's switch back to the \(Text\) Editor subview.

### How a Loom works

> Recommended reading: [Language models are multiverse generators](https://generative.ink/posts/language-models-are-multiverse-generators/)

Tapestry Loom is an implementation of a [Loom](https://generative.ink/posts/loom-interface-to-the-multiverse/), which is a multiversal tree-based interface. For those who have never used a Loom or a base model before, here's how they work:
1. You start with a snippet of text that you would like the model to complete.
	- Unlike "assistant" LLMs, base model LLMs are pure text predictors. You prompt them with text you'd like them to complete (such as "i wonder what a playground angel"), not queries (like "write a short story about the lives of the angels that linger around the playground").
2. The model generates numerous possible completions, and you select the completion you prefer the most. If you don't like any of the completions, you can write something in yourself.
3. This new completion is added to the document, and the cycle repeats, slowly building out a tree of possibilities.
	- At any time, you can backtrack and explore a different part of the tree.

In Tapestry Loom, these snippets are referred to as nodes. Nodes are referred to as having "parents" and "children", and can be active (part of the active text) or inactive (stored in the tree but not part of the active text).

### Using the Tree and \(Text\) Editor subviews



### Using the Bookmarks subview

### Saving your weave to disk

## Managing weaves

## Advanced editor usage

In this section, we'll explore the power user features of Tapestry Loom.

<!--
Stuff to bring up:
- Alternate subviews
- Reorganizing subviews
- Cursors and hovered nodes
- Automatic scrolling
- Keyboard shortcuts
- Shift-click and context menus
- Hover information
- Menu and metadata
	- Logprob nodees
-->