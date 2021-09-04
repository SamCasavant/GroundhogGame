class ThingMixin:
    def lookAt(self):
        print(self.name)
        if self.name != self.description:
            print(self.description)
        for tag in self.tags:
            if tag in ["eat", "take", "drink"]:
                print(f"I can {tag} this.")
            elif tag in ["take_req"]:
                print("I have to have this in my inventory to use it.")


"""    def make(self, inventory):
        #checks a given inventory for the necessary ingredients, returns used ingredients or None
        used = []
        if self.recipe:
            for ingredient in recipe:
                if ingredient in inventory: #Probably not right!
                    used.append(inventory[??]) #????
                else:
                    return "I do not have **ingredient**"
            return used
        else:
            return "**thing** cannot be made."
"""


class OpenMixin:
    def lock(self, key, actor):
        if not self.closed:
            return [
                0,
                [f"{actor.name} cannot lock an open door.", actor],
                actor.location,
            ]
        else:
            if self.lockable:
                if not self.locked:
                    self.locked = True

    def unlock(self, key, actor):
        if self.locked:
            if key == self.key:
                self.locked = False
            elif "pick" in key.tags:
                if "pickable" in self.tags:
                    self.locked = False
            else:
                print(f"{key.name} doesn't unlock this door.")
        else:
            print("This door is already unlocked.")

    def open(self, actor):
        if self.locked:
            if self.key in actor.inventory:
                print(f"I'll have to unlock it first with {self.key.name}")
            else:
                print(f"I'll need to unlock this door, but I don't have the key.")
        else:
            if self.closed:
                self.closed = False
                self.connection.unblock()
            else:
                print("This door is already open.")

    def close(self):
        if not self.closed:
            self.closed = True
            self.connection.block("The door is shut.")
        else:
            print("This door is already closed.")


class Thing(ThingMixin):
    def __init__(self, name, description=False, tags=[], recipe=None):
        self.name = name
        if description:
            self.description = description
        else:
            self.description = name
        self.tags = tags
        self.recipe = recipe


class Door(ThingMixin, OpenMixin):
    def __init__(
        self,
        name,
        connection,
        description=False,
        tags=["door", "openable", "pickable"],
        key=False,
        lockable=True,
        locked=False,
        closed=True,
    ):
        self.name = name
        self.tags = tags
        self.lockable = lockable
        self.locked = locked
        self.closed = closed
        self.connection = connection
        if lockable:
            if key:
                self.key = key
        if self.closed:
            connection.block("The door is closed.")


class Container(ThingMixin, OpenMixin):
    def __init__(
        self,
        name,
        contents=[],
        description=False,
        tags=["container", "pickable"],
        key=False,
        lockable=True,
        locked=False,
        closed=True,
    ):
        self.name = name
        self.contents = contents
        for item in self.contents:
            item.tags.append("contained")
        self.tags = tags
        self.lockable = lockable
        self.locked = locked
        self.closed = closed
        if lockable:
            if key:
                self.key = key
            else:
                print("Initialization Error: Lockable containers require a key.")

    def withdraw(self, thing, actor):
        if thing in self.contents:
            if len(actor.inventory) < actor.max_inv:
                actor.inventory.append(thing)
                self.contents.remove(thing)


class Food(ThingMixin):
    def __init__(self, name, description=False, tags=None, eat_val=4):
        if tags is None:
            tags = ["eat"]
        elif "eat" not in tags:
            tags.append("eat")
        self.name = name
        if description:
            self.description = description
        else:
            self.description = name
        self.tags = tags
        self.eat_val = eat_val


class Drink(ThingMixin):
    def __init__(self, name, description=False, tags=None, drink_val=2):
        if tags is None:
            tags = ["drink"]
        elif "drink" not in tags:
            tags.apend("drink")
        self.name = name
        if description:
            self.description = description
        else:
            self.description = name
        self.tags = tags
        self.drink_val = drink_val


class Recipe:
    # things composed of other things
    def __init__(self, ingredients, output):
        self.ingredients = ingredients
        self.output = output
