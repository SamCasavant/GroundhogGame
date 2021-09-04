class Output:
    def __init__(self, actor, verb, success, observer, verb_object=None, location=None):
        # An 'actor' 'verb's a 'verb_object'[optional], witnessed by 'observer'. 
        # Success may be a boolean or contain an explanation for why an action failed to proceed.
        if success != True && success != False:
            self.success = False
            self.reason = self.success
        else:
            self.success = success
            self.reason = None

        if location != None:
            self.location = location
        else:
            self.location = actor.location

        self.text = "Placeholder Text"
        self.actor = actor
        self.verb = verb
        self.observer = observer
        self.object = verb_object

        self.text = f"{self.getSubjectWord()} {self.verb}s "

        else:
            raise (f"Verb is not defined: {self.verb}")

    def getSubjectWord(self):
        if self.actor == self.observer: # Am I the one taking an action?
            return "I"
        elif self.actor == curSubject: # Are we allowed to use pronouns? Yes if the subject matches the previous sentence's.
            if self.actor.gender == "male":
                return "he"
            elif self.actor.gender == "female":
                return "she"
            else:
                return "they"
        elif self.actor.name in self.observer.known_names: # Does the viewer know the actor's name?
            return self.actor.name
        else: 
            return self.actor.impression

    def getVerb(self):
        if self.object:
            if self.verb == "move":
                return "moves to"
            if self.verb == "take":
                return "picks up"

def actToText(self, player, result, cue):
    # Dissassemble result array for legibility
    actor = result[0]
    name = actor.name()
    verb = result[1]
    targets = result[2]
    locations = result[3]
    successBool = result[4]
    extraString = result[5]
    
    return text

def outputVisible(self, player, locations):
    visibleLocations = []
    for location in locations:
        visibleLocations.append(location)
        for connection in location.connections:
            if connection.visible:
                visibleLocations.append(connection.getDest(location))
    for location in visibleLocations:
        if location == player.location:
            return True

def outputAudible(self, player, locations):
    audibleLocations = []
    for location in locations:
        audibleLocations.append(location)
        for connection in location.connections:
            if connection.audible:
                audibleLocations.append(connection.getDest(location))
    for location in audibleLocations:
        if location == player.location:
            return True

def listToNatural(self, pyList):
    output = ""
    if len(pyList) == 0:
        return output
    if len(pyList) == 1:
        return pyList[0]
    tempList = []
    for item in pyList:  # Count duplicates
        try:
            index = [each[0] for each in tempList].index(item)
            tempList[index][1] += 1
        except ValueError:
            tempList.append([item, 1])
    if len(tempList) == 1:
        return f"{item[1]} {item[0]}s."

    for item in tempList:
        if item == tempList[-1]:  # When we reach the end of the list
            if item == tempList[1]:  # If the list only had two items
                output = output[:-2] + " "  # drop the oxford comma
            output += "and "
        if item[1] == 1:
            output += f"{item[0]}, "
        else:
            output += f"{item[1]} {item[0]}s, "
    return output[:-2]  # delete trailing comma and space