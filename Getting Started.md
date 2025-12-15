# Getting Started

Tapestry Loom consists of three types of views: The file manager, (application-wide) settings, and editors. Unlike the other two views, editors contain multiple subviews and you can have an arbitrary number of editors open.

Editors can either be temporary, or they can be backed by a file on disk (Tapestry Loom's document format has the file extension `.tapestry`). When an editor is backed by a file on disk, changes will be automatically saved to disk. This way, if the application crashes, only very recent changes will be lost.

Note: If you'd like to test out Tapestry Loom's interface without configuring an inference provider, see the [example-weaves](./example-weaves/) folder for some sample documents.

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

### Using the Tree and \(Text\) Editor

The weave editor has many different subviews for visualizing the tree in different ways. In this section, we'll learn to use the Tree and \(Text\) Editor subviews.

The Editor subview allows you to edit the active text of the document. It is automatically updated as the active nodes change, and any changes made in this subview are automatically applied to the tree.

The Tree subview displays the nearby nodes in an interactive treelist. You can click on a node to activate it and you can hover over the node to perform common actions at that point (such as generating more nodes, creating a blank child node, and bookmarking or deleting the hovered node).

Nodes in all of the subviews are color coded by the label color of the model that produced them (if any), and the tokens within generated nodes are shaded by their probabilities (higher probability tokens are more opaque).

In order to start weaving, we'll need a node to continue from. So, let's write something in the text editor for the model to complete!

The possibilities are limitless, as base models were trained on such a large corpus of text that they can make a decent attempt at continuing pretty much anything you can think of. However, for those who find coming up with their first prompt intimidating, here's an example prompt:

```
welcome to the world you never knew existed.

so, how does it feel to step into the unknown?
```

After you have a node in the tree view, hover over it and click the speech bubble icon with a robot in it to generate some completions. If you're not satisfied with the completions you initially get, you can hover over the node he want to continue from and click the icon again to generate more completions.

Once you have a completion that you like, click on it to activate it and then hover over it and click the completion button again. If you want to go back, click the node's parent in the tree to go up.

Other things to try:
- Try using the other buttons that appear when you hover over a node. You can hover over a button to see a tooltip explaining it does.
- Try right clicking on a node to open it's context menu.
- Try hovering over the *text* of a node in order to view the metadata that it contains.
	- Try doing this in both subviews!

### Using the Bookmarks subview

When hovering over a node in the tree view, you can bookmark it by clicking the button with a bookmark icon. This node will appear as bookmarked in the tree, and you can repeat the steps taken to bookmark it to remove the bookmark.

Bookmarks are useful for quickly navigating to specific nodes in the weave. The Bookmarks subview contains a list of all bookmarks within your weave, in the order that they were added to the bookmarks list.

In the Bookmarks subview, you can:
- Click on a bookmark to activate the bookmarked node
- Right click on a bookmarked node to open it's context menu
- Remove a bookmark from the list by hovering over the node you'd like to remove and then clicking on the bookmark icon with a minus within it

### Saving your weave to disk

At the moment, your weave is considered temporary; If you exited Tapestry Loom without saving it, the contents would be lost.

In order to save your weave to disk, click on the "Save as..." button in the bottom left of the weave editor. This will open a dialog where you can enter a name for your weave before saving it to disk.

After your weave is saved to disk, the bottom left will now display the path your weave is stored at. All paths in the UI are displayed as being relative to Tapestry Loom's root location (defaults to "~/Documents/Tapestry Loom", can be changed in Settings > Document > Root location).

Now that your weave has been saved, you can safely close the editor view.

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