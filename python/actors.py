import space
import things
import random


"""Time is defined here for the time being, fix later!"""
class Chrono:  # Time, in seconds since midnight
    def __init__(self, start, step=60):
        self.time = start
        self.step = step

    def tick(self, time=0):
        if time:
            self.time += time
        else:
            self.time += self.step
        return self.time

    def convertToSeconds(self, time, format):
        if format == "second":
            return time
        elif format == "minute":
            return time * 60
        elif format == "hour":
            return time * 3600

    def getTime(self, format):
        if format == "second":
            return self.time
        elif format == "minute":
            return self.time / 60
        elif format == "hour":
            return self.time / 3600
        elif format == "clock":
            ampm = "AM"
            hours = floor(self.time / 3600)
            minutes = floor((self.time - (3600 * hours)) / 60)
            seconds = floor(self.time - (3600 * hours) - (60 * minutes))
            if hours > 12:
                hours = hours - 12
                ampm = "PM"
            if minutes < 10:
                minutes = f"0{minutes}"
            if seconds < 10:
                seconds = f"0{seconds}"
            return f"{hours}:{minutes}:{seconds} {ampm}"

    def timedRandom(resolution, range):
        pass


actorList = []
TIME = Chrono(33600, step=(1 / 100))  # Start time at ~9AM, 1/100 second at a time)
player = None


class ActorMixin:  # This is a real class actor.
    """This class implements a set of basic functions that apply to all of the agents in the game."""

    def name(self, formal=False):
        return self.getName(formal)

    def update(self):
        self.updateStatus()

    def act(self):
        """Takes actions and returns a result of the form [successBool, [output, actor, verb]]"""
        plan = self.createPlan()
        result = self.executePlan(plan)
        return result

    def executePlan(self, plan):
        if len(plan) > 0:
            action = plan[0]
            if action[0] == "move":
                if issubclass(type(action[1]), space.SpaceMixin):
                    result = self.move(action[1])
                elif type(action[1]) == space.Connection:
                    result = self.move(action[1].getDest(self))
            elif action[0] == "take":
                result = self.take(action[1])
            elif action[0] == "eat":
                result = self.eat(action[1])
            elif action[0] == "wait":
                result = [self, "wait", [], [self.location], 1, ""]
            else:
                result = [
                    f"{self.properName} attempts to {action[0]} but doesn't know how.",
                    self,
                    None,
                ]
            if action[:-1] in [item[1] for item in self.itinerary]:
                index = [item[1] for item in self.itinerary].index(action[:-1])
                del self.itinerary[index]
        return result

    # Actions return an array of [actor, verb, [targets], [locations], successBool, extraString] which is processed to text in output.py
    def move(self, destination):
        locations = [self.location, destination]
        self.location.actors.remove(self)
        self.location = destination
        self.location.actors.append(self)
        self.tAction = TIME.getTime("second") + 60 / self.speed
        return [
            self,
            "move",
            locations,
            locations,
            1,
            "",
        ]  # Move is a special case where targets = locations

    def take(self, thing):
        if self.canTakeThing(thing):
            self.tAction = TIME.getTime("second") + 30 / self.speed
            self.takeThing(thing)
            self.inventory.append(thing)
            successBool = 1
        else:
            successBool = 0
        return [self, "take", [thing], [self.location], successBool, ""]

    def eat(self, thing):
        if self.canEatThing(thing):
            self.tAction = TIME.getTime("second") + 300 / self.speed
            self.eatThing(thing)
            successBool = 1
        else:
            successBool = 0
        return [self, "eat", [thing], [self.location], successBool, ""]

    def drink(self, thing):
        if self.canDrinkThing(thing):
            self.tAction = TIME.getTime("second") + 30 / self.speed
            self.drinkThing(thing)
            successBool = 1
        else:
            successBool = 0
        return [self, "drink", [thing], [self.location], successBool, ""]

    def canEatThing(self, thing):
        if thing in self.inventory:
            if "eat" in thing.tags:
                return True
        elif thing in self.location.things:
            if "take_req" not in thing.tags:
                return True
            else:
                return False
        else:
            return False

    def eatThing(self, thing):
        if thing in self.inventory:
            self.inventory.remove(thing)
            self.states["eat"] -= thing.eat_val
        elif thing in self.location.things:
            if "permanent" in thing.tags:
                self.states["eat"] -= thing.eat_val
            else:
                self.location.removeThing(thing)
                self.states["eat"] -= thing.eat_val

    def canDrinkThing(self, thing):
        if thing in self.inventory:
            if "drink" in thing.tags:
                return True
        elif thing in self.location.things:
            if "take_req" not in thing.tags:
                return True
            else:
                return False
        else:
            return False

    def drinkThing(self, thing):
        if thing in self.inventory:
            self.inventory.remove(thing)
            self.states["drink"] -= thing.eat_val
        elif thing in self.location.things:
            if "permanent" in thing.tags:
                self.states["drink"] -= thing.eat_val
            else:
                self.location.removeThing(thing)
                self.states["drink"] -= thing.eat_val

    def canTakeThing(self, thing):
        if "take" in thing.tags:
            if len(self.inventory) < self.max_inv:
                return True

    def takeThing(self, thing):
        self.inventory.append(thing)
        self.location.removeThing(thing)


class AnimalPhysicalMixin:
    """This class implements basic animal functions like hunger and thirst."""

    def getName(self, formal=False):
        if formal:
            return self.properName
        else:
            return self.commonName

    def updateStatus(self):
        self.states["eat"] += 0.1 * self.hunger_rate
        self.states["drink"] += 0.1 * self.thirst_rate

    def talk(self):
        return [f"{self.properName} {random.choice(self.sounds)}s loudly."]


class AnimalAIMixin:
    def createPlan(self):
        plan = []
        # Choose actions that result from status
        for state in self.states.keys():
            if self.strategies[state] == "search":
                find = self.search(state)
                if find:
                    plan.append(find)
                else:
                    if "move" not in [plan[0] for plan in plan]:
                        move = self.findMove()
                        if move:
                            plan.append(["move", move, self.states[state]])
        plan.append(["wait", self.lazyThreshold])
        plan.sort(key=lambda x: x[-1], reverse=True)
        return plan

    def findMove(self):
        moves = []
        for connection in self.location.connections:
            if connection.blocked == False:
                moves.append(connection)
        if moves:
            return random.choice(moves)
        else:
            return False

    def getStrategy(self, state):
        try:
            strategy = self.strategies[state]
            return strategy
        except ValueError:
            return "search"

    def search(self, state):
        for thing in self.inventory:
            if state in thing.tags:
                return [state, thing, self.states[state]]
        else:
            for thing in self.location.things:
                if state in thing.tags:
                    if "take_req" in thing.tags:
                        if "take" in thing.tags:
                            return ["take", thing, self.states[state]]
                    else:
                        return [state, thing, self.states[state]]
            else:
                return None


class HumanPhysicalMixin:
    """This class implements physical properties that are applicable to humans."""

    def getName(
        self, formal=True
    ):  # Only different from animal getName by defaulting to formal.
        if formal:
            return self.properName
        else:
            return self.commonName


class HumanAIMixin:
    """This class implements a set of higher level planning and actions."""

    def createPlan(self):
        """Finds all actions that should be taken, sorts by priority.
        Actions are a list starting with a verb, followed by any objects, and concluding with priority."""
        plan = [["wait", self.lazyThreshold]]
        # Choose actions that result from status
        for state in self.states.keys():
            if self.strategies[state] == "search":
                find = self.search(state)
                if find:
                    action = [state, find, self.states[state]]
                    plan.append(action)
            elif self.conditions[self.strategies[state]]:
                action = self.special(self.strategies[state], self.states[state])
                if action:
                    plan.append(action, self.states[state])
            else:  # If we can not solve the problem, move around.
                move = self.findMove()
                if move:
                    plan.append(["move", move, self.states[state]])
        # Choose actions that result from long term plan
        if self.itinerary:
            plan.append(
                [
                    self.itinerary[0][1][0],
                    self.itinerary[0][1][1],
                    1000 / (self.itinerary[0][0] - TIME.time + 1),
                ]
            )
        plan.sort(key=lambda x: x[-1], reverse=True)
        return plan

    def findMove(self):
        """Finds possible moves from current location."""
        moves = []
        for connection in self.location.connections:
            if connection.blocked == False:
                moves.append(connection)
        if moves:
            return random.choice(moves)
        else:
            return False

    def addItinerary(self, itinerary):
        """Itinerary events are preplanned actions to be taken at a specified time, of the form [time, action]"""
        for item in itinerary:
            self.itinerary.append(item)
        self.itinerary.sort()

    def getStrategy(self, state):
        """Given a state that needs to be accounted for, return the actor's strategy for resolving it."""
        try:
            strategy = self.strategies[state]
            return strategy
        except ValueError:
            return "search"

    def search(self, thing):
        """Searches inventory and environment for either a specific thing or one with a property."""
        if type(thing) == str:
            for thing2 in self.inventory:
                if thing in thing2.tags:
                    return thing2
            else:
                for thing2 in self.location.things:
                    if thing in thing2.tags:
                        return thing2
                else:
                    return None
        elif type(thing) == things.Thing:
            if thing in self.inventory:
                return thing
            elif thing in self.location.things:
                return thing

    def special(self, action, priority):
        """For actions that are composed of other actions, returns the next steps."""
        plan = []
        ready = True
        for condition in self.conditions[action]:
            if self.isMet(condition):
                pass
            elif condition[1] < priority:
                pass
            else:
                ready = False
                # plan.append(self.meetCondition(condition)[:-1].append(priority))
        if ready:
            pass
        else:
            return plan

    def isMet(self, condition):
        if condition[0] == "have":
            if condition[1] in self.inventory:
                return True
            else:
                return False

    def meetCondition(self, condition):
        """Relates conditions to required actions."""
        if condition[0] == "have":
            if type(condition[1]) == str:
                find = self.search(condition[1])


class Human(ActorMixin, HumanPhysicalMixin, HumanAIMixin, AnimalPhysicalMixin):
    """Properties:
    States: A dictionary of potential causes of action and their degree of intensity, labeled according to the relevant verb (instead of 'hungry', 'eat') for internal consistency (where possible).
    Strategies: A dictionary that relates states to response strategies; only search is currently implemented.
    Itinerary: A list of planned actions and the time, in seconds since midnight, that it is supposed to occur by.
    Tags: A miscellanious collection of properties that impact what a character can do and what can be done to them.
    lazyThreshold: The priority given to 'wait' in the plan.
    Conditions: Prerequisites for strategies of the form ['property', 'value', 'priority'], eg. Grumph should have a weapon and would like an associate before attempting to mug, so conditions['mug']=[['have', 'weapon', 100], ['be', 'cooperating', 10]
    tAction: The time at which the actor can take another action."""

    def __init__(
        self,
        properName,
        commonName="person",
        description="A regular human being.",
        inventory=None,
        max_inv=5,
        hunger_rate=1,
        hunger=3,
        itinerary=None,
        tags=None,
        thirst=3,
        thirst_rate=1,
        strategies=None,
        lazyThreshold=5,
        tAction=0,
        speed=1,
    ):
        if inventory is None:
            inventory = []
        if itinerary is None:
            itinerary = []
        if tags is None:
            tags = ["human"]
        if strategies is None:
            strategies = {"eat": "search", "drink": "search", "money": "search"}
        self.plan = []
        self.states = {"eat": hunger, "drink": thirst}
        self.properName = properName
        self.commonName = commonName
        self.description = description
        self.inventory = inventory
        self.max_inv = max_inv
        self.hunger_rate = hunger_rate
        self.thirst_rate = thirst_rate
        self.tags = tags
        self.strategies = strategies
        self.lazyThreshold = lazyThreshold
        self.itinerary = itinerary
        self.tAction = tAction
        self.speed = speed
        if itinerary:
            self.itinerary = itinerary.sort()
        else:
            self.itinerary = []
        actorList.append(self)


class User(ActorMixin, HumanPhysicalMixin, AnimalPhysicalMixin):
    def __init__(
        self,
        properName,
        commonName="me",
        description="This is the person that I am.",
        inventory=[],
        max_inv=10,
        hunger_rate=1,
        hunger=3,
        thirst_rate=1,
        thirst=3,
        tags=["human", "user"],
        tAction=0,
        speed=1,
    ):
        self.properName = properName
        self.commonName = commonName
        self.description = description
        self.inventory = inventory
        self.max_inv = max_inv
        self.states = {"eat": hunger, "drink": thirst}
        self.hunger_rate = hunger_rate
        self.thirst_rate = thirst_rate
        self.tags = tags
        self.action = ["wait"]
        self.tAction = tAction
        self.speed = speed
        actorList.append(self)

    def look(self):
        text = ""
        text += f"I am standing in {self.location.name}. {self.location.description}\n"
        if len(self.location.actors) > 2:
            verbIs = "are"
        else:
            verbIs = "is"
        if len(self.location.actors) > 1:
            text += f"{output.listToNatural([actor.properName for actor in self.location.actors if 'user' not in actor.tags])} {verbIs} here.\n"
        if len(self.location.things) > 0:
            text += f"I can see {output.listToNatural([thing.name for thing in self.location.things])}.\n"
        if len(self.location.connections) > 1:
            for connection in self.location.connections:
                text += f"I can go {connection.getDir(player)} to {connection.getDest(player).name}."
        return text

    def chk_inventory(self):
        if self.inventory:
            text = "I have:"
            for thing in self.inventory:
                text += thing.name + "\n"
        else:
            text = "I don't have anything at the moment."
        return text

    def chk_states(self):
        text = ""
        for state in self.states.keys():
            text += f"{state}: {self.states[state]}.\n"
        return text

    def act(self):
        if self.action[0] == "move":
            result = self.move(self.action[1].getDest(self))
        elif self.action[0] == "take":
            result = self.take(self.action[1])
        elif self.action[0] == "look":
            result = [self.look(), self, "look"]
        elif self.action[0] == "look at":
            result = [self.action[1].lookAt(), self, "look"]
        elif self.action[0] == "open":
            self.action[1].open(self)
            result = [f"I open {self.action[1].name}", self, "open"]
        elif self.action[0] == "close":
            self.action[1].close()
            result = [f"I close {self.action[1].name}", self, "close"]
        elif self.action[0] == "inventory":
            result = [self.chk_inventory(), self, "chk_inv"]
        elif self.action[0] == "wait":
            result = [self, "wait", [], [self.location], 1, ""]
        self.action = ["wait"]
        return result


class HouseCat(ActorMixin, AnimalPhysicalMixin, AnimalAIMixin):
    """Properties:
    States: A dictionary of potential causes of action and their degree of intensity, labeled according to the relevant verb (instead of 'hungry', 'eat') for internal consistency.
    Tags: A miscellanious collection of properties that impact what a character can do and what can be done to them.
    lazyThreshold: The priority given to 'wait' in the plan."""

    def __init__(
        self,
        commonName="Cat",
        properName=None,
        description=None,
        inventory=None,
        max_inv=1,
        hunger_rate=1,
        hunger=3,
        thirst_rate=1,
        thirst=3,
        tags=None,
        sounds=None,
        lazyThreshold=5,
        tAction=0,
        speed=1,
    ):
        if inventory is None:
            inventory = []
        if itinerary is None:
            itinerary = []
        if tags is None:
            tags = ["human"]
        if sounds is None:
            sounds = ["meow", "purr"]
        if not properName:
            self.properName = commonName
        else:
            self.properName = properName
        self.commonName = commonName
        self.description = description
        self.inventory = inventory
        self.max_inv = max_inv
        self.states = {"eat": hunger, "drink": thirst}
        self.hunger_rate = hunger_rate
        self.thirst_rate = thirst_rate
        self.tags = tags
        self.sounds = sounds
        self.lazyThreshold = lazyThreshold
        self.tAction = tAction
        self.speed = speed
        actorList.append(self)
