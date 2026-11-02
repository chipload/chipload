# Groups

A **group** keeps drawn shapes and texts together: a sign's frame, border and words, or a
logo inserted from the [shape library](shape-library.md). One click selects all of it, and
it moves, turns, scales and is deleted as one. An operation made from a group cuts what is
in the group, also shapes added to it later. (Design: `docs/specs/shape-groups.md`.)

What can be in a group: drawn shapes and texts (a text whole, with its letters). A DXF's
shapes and a traced picture's already belong to their file or picture, which keeps them
together; explode the DXF, or put the picture through the shape library, to group its
shapes. A shape is in one group at most, and groups don't hold groups.

Inserting a library entry of more than one shape or text makes a group named after it.

## Cutting a group

Select the group and choose an operation, V-carve for example: it cuts the group as it is
each time, so a shape added to the group later is cut too, and one taken out isn't.
Ctrl+click on shapes of the selected group leaves them out of the selection and back in,
as for a picture or a DXF: the operation cuts the rest. In the project file the operation's
selection is `{"group": 1}`, or `{"group": 1, "exclude": [7]}`.

Ungrouping keeps what an operation made from the group cuts: it gets the group's shapes by
id. A group with texts in it can't be ungrouped while an operation cuts it (an operation
can't name a text's letters by id): choose that operation's shapes again first, or take the
texts out of the group.

## On the command line

```sh
chipload group add sign.chipload 5 6 --texts 1 --name Sign   # prints the group's id
chipload group list sign.chipload
chipload group set sign.chipload 1 --name "Door sign" --add 9,10 --add-texts 2 --remove 6 --remove-texts 3
chipload group move sign.chipload 1 --dx 10 --dy 0
chipload group turn sign.chipload 1 --angle-deg 90           # about its middle, or --about X,Y
chipload group scale sign.chipload 1 --factor 0.5            # its texts' height scales too
chipload group remove sign.chipload 1                        # ungroup: the shapes stay, free
chipload group delete sign.chipload 1                        # the shapes and texts too
```

`group add` with shapes of one group among them adds the rest to that group; with shapes of
several groups, those groups become one.
