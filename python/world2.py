import actors
import things
import space
from copy import copy


def worldInit():
    # things
    pie = things.Food("pie", "A freshly baked pie.", tags=["take", "eat"], eat_val=10)
    knife = things.Thing("knife", "Sharp as all heck.", tags=["take"])

    # Actors
    GrumphTorgi = actors.Human("Grumph Torgi", "Grumph", "A villain!")
    SilbertHumperdinck = actors.Human(
        "Silbert Humperdinck", "Sil", "Looks like a respectable fellow."
    )
    GertyVanFleek = actors.Human(
        "Gerty Van Fleek", "Gerty", "An old pie woman of some sort.", inventory=[pie]
    )
    MelissaMansname = actors.Human("Melissa Mansname", "Mel", "Just wed; nee Forthod")
    UmbrellaDeVille = actors.Human(
        "Umbrella DeVille", "Ella", "Should be named deMaitreDe."
    )
    # Spaces
    Alley = space.Road("alley", "A dark alley.")
    Park = space.Room("park", "Look at this grass.")
    TorgiHome = space.Room(
        "Torgi Household", "The cluttered home of a man named Torgi."
    )
    MansnameHome = space.Room(
        "Mansname Household", "Wow! It's hard to come up with descriptions!"
    )
    VanFleekHome = space.Room("Van Fleek Household", "Reeks of pie.")
    Restaurant = space.Room("Barren Grille", "I hear they have great desert.")
    # Spaces -Connections
    space.Connection(TorgiHome, Park, "north", "south")
    space.Connection(TorgiHome, Restaurant, "west", "east")
    space.Connection(Restaurant, Park, "north", "south")
    space.Connection(Park, Alley, "north", "south")
    space.Connection(VanFleekHome, Alley, "west", "east")
    space.Connection(MansnameHome, Alley, "east", "west")
    # Spaces -things
    tempKnife = copy(knife)
    TorgiHome.addThings([tempKnife])
    Restaurant.addThings([copy(pie)])

    # Spaces -Actors
    TorgiHome.addActors([GrumphTorgi, SilbertHumperdinck])
    Restaurant.addActors([UmbrellaDeVille])
    VanFleekHome.addActors([GertyVanFleek])
    MansnameHome.addActors([MelissaMansname])
    # Actors -Itinerary
    GrumphTorgi.addItinerary(
        [
            (36000, ["take", tempKnife]),
            (43200, ["move", Park]),
            (57600, ["move", Alley]),
        ]
    )
    # SilbertHumperdinck.addItinerary([(25200, 'wake up'), ()])
    # Player
    player = actors.User("User")
    Park.addActors([player])
    actors.player = player
    return player
