import things


class SpaceMixin:
    def addActors(self, actors):
        for actor in actors:
            self.actors.append(actor)
            actor.location = self

    def removeActor(self, actor):
        if actor in self.actors:
            self.actors.remove(actor)

    def addThings(self, things):
        for thing in things:
            self.things.append(thing)
            thing.location = self

    def removeThing(self, thing):
        if thing in self.things:
            self.things.remove(thing)

    def addConnection(self, destination, dir_a, dir_b, visible=0, audible=1):
        if destination in [connection.getDest() for connection in self.connections]:
            print("Connection already exists.")
        else:
            Connection(self, destination, dir_a, dir_b, visible, audible)


class Connection:
    def __init__(
        self, loc_a, loc_b, dir_a, dir_b, visible=0, audible=1,
    ):
        self.loc_a = loc_a
        self.loc_b = loc_b
        self.dir_a = dir_a
        self.dir_b = dir_b
        loc_a.connections.append(self)
        loc_b.connections.append(self)
        self.visible = visible
        self.audible = audible
        self.blocked = False

    def actorDest(self, current):
        if actor.location == self.loc_a:
            return self.loc_b
        elif actor.location == self.loc_b:
            return self.loc_a

    # def connectionDest(self, connection):

    def getDir(self, actor):
        if actor.location == self.loc_a:
            return self.dir_a
        elif actor.location == self.loc_b:
            return self.dir_b

    def getDest(self, actor):
        if actor.location == self.loc_a:
            return self.loc_b
        elif actor.location == self.loc_b:
            return self.loc_a

    def unblock(self):
        self.blocked = False

    def block(self, blockreason=""):
        self.blocked = True
        self.blockreason = blockreason


def findPath(
    starts, ends, visitedNodes=[]
):  # takes a list of starting points and ending points, returns a path
    startMoves = []  # Locations that can be accessed from starts
    endMoves = []  # Locations that can be accessed from ends
    # First: See if any starts connect to any ends; build list of startMoves:
    for loc_a in starts:
        for move in [connection.getDest(loc_a) for connection in loc_a.connections]:
            if move in ends:
                return [loc_a, move]
            else:
                if move not in visitedNodes:
                    startMoves.append(move)
                    visitedNodes.append(move)
    # Second: See if there is a midpoint that connects any starts to any ends; build list of endMoves:
    for loc_b in ends:
        for move in [connection.getDest(loc_b) for connection in loc_b.connections]:
            if move in startMoves:
                for loc_a in starts:
                    if move in [
                        connection.getDest(loc_a) for connection in loc_a.connections
                    ]:
                        return [loc_a, move, loc_b]
            else:
                if move not in visitedNodes:
                    endMoves.append(move)
                    visitedNodes.append(move)
    # Third: (Recursive) Try to find path between startMoves and endMoves; build path:
    interPath = findPath(startMoves, endMoves, visitedNodes)
    for loc_a in starts:
        if interPath[0] in [
            connection.getDest(loc_a) for connection in loc_a.connections
        ]:
            path = [loc_a]
            break
    for loc_b in ends:
        if interPath[-1] in [
            connection.getDest(loc_b) for connection in loc_b.connections
        ]:
            path.append(loc_b)
            break
    index = 1
    for element in interPath:
        path.insert(index, element)
        index += 1
    return path


class Road(SpaceMixin):
    def __init__(
        self,
        name="Road",
        description="Gets you from here to there.",
        visibleLocations=None,
        audibleLocations=None,
        tags=None,
    ):
        if visibleLocations is None:
            visibleLocations = [self]
        elif self not in visibleLocations:
            visibleLocations.append(self)
        if audibleLocations is None:
            audibleLocations = []
        elif self not in audibleLocations:
            audibleLocations.append(self)
        if tags is None:
            tags = ["exterior"]
        elif "exterior" not in tags:
            tags.append("exterior")
        self.name = name
        self.description = description
        self.visibleLocations = visibleLocations
        self.audibleLocations = audibleLocations
        self.connections = []
        self.actors = []
        self.things = []


class Bedroom(SpaceMixin):
    def __init__(
        self,
        name="Bedroom",
        description="A place to sleep.",
        visibleLocations=None,
        audibleLocations=None,
        bed=None,
        bureau=None,
    ):
        if visibleLocations is None:
            visibleLocations = [self]
        elif self not in visibleLocations:
            visibleLocations.append(self)
        if audibleLocations is None:
            audibleLocations = []
        elif self not in audibleLocations:
            audibleLocations.append(self)
        if tags is None:
            tags = ["interior"]
        elif "interior" not in tags:
            tags.append("interior")
        if bed is None:
            bed = things.Thing("bed", description="Comfy", tags=["sleep"])
        if bureau is None:
            bureau = things.Container("bureau", description="Drawers for drawers.")
        self.name = name
        self.description = description
        self.visibleLocations = visibleLocations
        self.audibleLocations = audibleLocations
        self.connections = []
        self.actors = []
        self.bed = bed
        self.bureau = bureau
        self.things = [self.bed, self.bureau]


class Kitchen(SpaceMixin):
    def __init__(
        self,
        name="Kitchen",
        description="Cooking and eating.",
        visibleLocations=None,
        audibleLocations=None,
        bed=None,
        bureau=None,
        tags=None,
    ):
        if visibleLocations is None:
            visibleLocations = [self]
        elif self not in visibleLocations:
            visibleLocations.append(self)
        if audibleLocations is None:
            audibleLocations = []
        elif self not in audibleLocations:
            audibleLocations.append(self)
        if tags is None:
            tags = ["interior"]
        elif "interior" not in tags:
            tags.append("interior")
        if stove is None:
            stove = things.Thing("stove", description="Hot hot heat.", tags=["cook"])
        if fridge is None:
            fridge = things.Container(
                "fridge", description="Keeps cold things cold and hot things cold."
            )
        if sink is None:
            sink = things.Drink(
                "sink",
                description="We just need everything else now.",
                tags=["permanent", "drink", "wash"],
            )
        self.name = name
        self.description = description
        self.visibleLocations = visibleLocations
        self.audibleLocations = (audibleLocations,)
        self.connections = []
        self.actors = []
        self.things = []
        self.fridge = fridge
        self.stove = stove
        self.sink = sink
        self.things = [self.fridge, self.sink, self.stove]


class Bathroom(SpaceMixin):
    def __init__(
        self,
        name="Bathroom",
        description="Bath, toilet, and beyond.",
        visibleLocations=None,
        audibleLocations=None,
        bath=None,
        toilet=None,
        sink=None,
        tags=None,
    ):
        if visibleLocations is None:
            visibleLocations = [self]
        elif self not in visibleLocations:
            visibleLocations.append(self)
        if audibleLocations is None:
            audibleLocations = []
        elif self not in audibleLocations:
            audibleLocations.append(self)
        if tags is None:
            tags = ["interior"]
        elif "interior" not in tags:
            tags.append("interior")
        if bath is None:
            bath = things.Thing("bath", description="A tub.", tags=["bathe", "wash"])
        if toilet is None:
            toilet = things.Thing("toilet", description="Drawers for drawers.")
        if sink is None:
            sink = things.Drink(
                "sink", description="", tags=["permanent", "drink", "wash"]
            )
        self.name = name
        self.description = description
        self.visibleLocations = visibleLocations
        self.audibleLocations = audibleLocations
        self.connections = []
        self.actors = []
        self.things = []
        self.fridge = fridge
        self.stove = stove
        self.sink = sink
        self.things = [self.fridge, self.sink, self.stove]


class Building:
    def __init__(self, rooms, layout):
        for room in rooms:
            for connection in layout[room]:
                if connection not in room.connections:
                    room.addConnection(
                        layout[room][0], layout[room][1], layout[room][2]
                    )


class House:
    def __init__(
        self, bedrooms=1, kitchens=1, bathrooms=1,
    ):
        self.bedroom = Bedroom()
        self.kitchen = Kitchen()


class Room(SpaceMixin):
    def __init__(
        self, name, description, visibleLocations=None, audibleLocations=None, tags=None
    ):
        if visibleLocations is None:
            visibleLocations = [self]
        elif self not in visibleLocations:
            visibleLocations.append(self)
        if audibleLocations is None:
            audibleLocations = []
        elif self not in audibleLocations:
            audibleLocations.append(self)
        if tags is None:
            tags = []
        self.name = name
        self.description = description
        self.visibleLocations = visibleLocations
        self.audibleLocations = audibleLocations
        self.connections = []
        self.actors = []
        self.things = []
