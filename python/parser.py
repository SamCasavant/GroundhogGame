verbs = ["move", "look", "take", "inventory", "wait", "talk", "open", "close"]
translations = [
    ("s", "south"),
    ("n", "north"),
    ("w", "west"),
    ("e", "east"),
    ("go", "move"),
    ("i", "inventory"),
    ("u", "up"),
    ("d", "down"),
]
prepositions = ["at", "to"]


def parse(player, command):  # produces actions of the form [verb, thing]
    words = translate(command.split(" "))
    for preposition in prepositions:
        if preposition in words:
            words = [word for word in words if word != preposition]
    if words[0] in verbs:
        if len(words) > 1:
            obj = determineObject(player, words[1:])
        if words[0] == "move":
            if obj in player.location.connections:
                return ["move", obj]
            else:
                print("I can't go %s from here." % words[1])
                return 0
        elif words[0] == "take":
            if obj in player.location.things:
                if "take" in obj.tags:
                    return ["take", obj]
                else:
                    print(f"I can't take {obj.name.lower()}")
            else:
                print("I don't see %s." % " ".join(words[1:]))
                return 0
        elif words[0] == "look":
            if len(words) == 1:
                return ["look"]
            if obj in player.location.actors or obj in player.location.things:
                return ["look at", obj]
            else:
                print("I don't see %s." % obj)
                return 0
        elif words[0] == "talk":
            if obj in player.location.actors:
                return ["talk", obj]
            elif obj in player.location.things:
                print(
                    f"You try striking up a conversation with {obj.name}, but it seems unresponsive."
                )
                return 0
            else:
                return 0
        elif words[0] == "open":
            if obj in player.location.things:
                if "openable" in obj.tags:
                    if obj.closed:
                        if obj.locked:
                            print(f"{obj.name} is locked.")
                            return 0
                        else:
                            return ["open", obj]
                    else:
                        print(f"{obj.name} is already open.")
                        return 0
                else:
                    print(f"{obj.name} cannot be opened.")
                    return 0
        elif words[0] == "close":
            if obj in player.location.things:
                if "openable" in obj.tags:
                    if not obj.closed:
                        return ["close", obj]
                    else:
                        print([f"{obj.name} is already closed."])
                        return 0
                else:
                    print(f"{obj.name} cannot be closed.")
                    return 0

        elif words[0] == "inventory":
            return ["inventory"]
        elif words[0] == "wait":
            return ["wait"]
    else:
        obj = determineObject(player, words)
        if obj in player.location.connections:
            return ["move", obj]


def translate(words):  # changes out synonyms and makes lowercase
    for index, word in enumerate(words):
        try:
            map_index = [trans[0] for trans in translations].index(word)
            words[index] = translations[map_index][1].lower()
        except ValueError:
            words[index] = words[index].lower()
            pass
    return words


def determineObject(player, words):
    words = " ".join(words)
    # is it a thing:
    try:
        index = [thing.name.lower() for thing in player.location.things].index(words)
        return player.location.things[index]
    except ValueError:
        # is it an actor (by proper name)
        try:
            index = [
                actor.properName.lower() for actor in player.location.actors
            ].index(words)
            return player.location.actors[index]
        except ValueError:
            try:
                index = [
                    actor.commonName.lower() for actor in player.location.actors
                ].index(words)
                return player.location.actors[index]
            except ValueError:
                # is it a direction
                try:
                    index = [
                        connection.getDir(player).lower()
                        for connection in player.location.connections
                    ].index(words)
                    return player.location.connections[index]
                except ValueError:
                    # is it a place
                    try:
                        index = [
                            connection.getDest(player).name.lower()
                            for connection in player.location.connections
                        ].index(words)
                        return player.location.connections[index]
                    except ValueError:
                        print(f"{words} not available.")
                        return 0
