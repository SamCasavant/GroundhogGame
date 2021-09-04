import actors
import things
import space
import copy
import actors


def worldInit():
    # things
    carKey = things.Thing("car key", tags=["take"])
    catFood = things.Thing("cat food", tags=["take", "eat", "cat food"], eat_val=6)
    # Actors
    hungryCat = actors.HouseCat(
        properName="Hungry Cat",
        commonName="Cat",
        description="He looks like he has a big appetite.",
        inventory=[catFood],
        hunger=15,
        hunger_rate=15,
    )
    # mouse = actors.Animal(
    #    commonName="mouse", description="It's a little mouse.", tags=[]
    # )
    # Spaces
    room27 = space.Space("Room 27", "My hotel room.")
    hallway = space.Space("the second floor hallway", "A long narrow hallway.")
    # Spaces -Exits
    doorway = space.Connection(room27, hallway, "south", "north", blocked=False)
    # Spaces -doors
    hallway.addThings([things.Door("door", doorway, "An old wooden door.", room27)])
    # Spaces -things
    room27.addThings([carKey])
    room27.addThings([copy.copy(catFood)])
    hallway.addThings([copy.copy(catFood), copy.copy(catFood), copy.copy(catFood)])
    # Spaces -Actors
    room27.addActors([hungryCat])
    # room27.addActors([copy.copy(mouse), copy.copy(mouse)])
    # Player
    player = actors.User("User", location=room27)
    actors.player = player
    return player
