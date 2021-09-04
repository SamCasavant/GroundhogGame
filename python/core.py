import world2
import actors
from parser import parse
import tkinter as tk
import time
import os
from PIL import ImageTk, Image
from math import floor, cos, pi

TIME = actors.TIME
Chrono = actors.Chrono
2

class Clock:
    def __init__(self, master):
        self.master = master
        self.canvas = tk.Canvas(master, width=400, height=400)

        self.lastUpdate = TIME.getTime("second")

        self.face = ImageTk.PhotoImage(Image.open("face.png"))
        self.gearcover = ImageTk.PhotoImage(Image.open("gearcover.png"))

        self.gear1 = Image.open("gear1.png")
        self.gear2 = Image.open("gear2.png")
        self.gear3 = Image.open("gear3.png")
        self.hourhand = Image.open("hourhand.png")
        self.minutehand = Image.open("minutehand.png")
        self.secondhand = Image.open("secondhand.png")

        self.tkgear1 = ImageTk.PhotoImage(self.gear1)
        self.tkgear2 = ImageTk.PhotoImage(self.gear2)
        self.tkgear3 = ImageTk.PhotoImage(self.gear3)
        self.tkhour = ImageTk.PhotoImage(self.hourhand)
        self.tkminute = ImageTk.PhotoImage(self.minutehand)
        self.tksecond = ImageTk.PhotoImage(self.secondhand)

        self.canvas_face = self.canvas.create_image(200, 200, image=self.face)
        self.canvas_gear1 = self.canvas.create_image(100, 100, image=self.tkgear1)
        self.canvas_gear2 = self.canvas.create_image(200, 100, image=self.tkgear2)
        self.canvas_gear3 = self.canvas.create_image(100, 150, image=self.tkgear3)
        self.canvas_hour = self.canvas.create_image(200, 200, image=self.tkhour)
        self.canvas_minute = self.canvas.create_image(200, 200, image=self.tkminute)
        self.canvas_second = self.canvas.create_image(200, 200, image=self.tksecond)
        self.canvas_gearcover = self.canvas.create_image(200, 200, image=self.gearcover)

    def update(self):
        self.canvas.delete(self.canvas_hour)
        self.canvas.delete(self.canvas_minute)
        self.canvas.delete(self.canvas_second)
        self.canvas.delete(self.canvas_gear1)
        self.canvas.delete(self.canvas_gear2)
        self.canvas.delete(self.canvas_gear3)
        self.canvas.delete(self.canvas_gearcover)
        seconds = TIME.getTime("second")
        # update hands no more than once per second
        if self.lastUpdate <= seconds - 1:
            hour_angle = (seconds % 43200) / 120
            minute_angle = (seconds % 3600) / 10
            second_angle = (floor(seconds) % 60) * 6
            self.tkhour = ImageTk.PhotoImage(self.hourhand.rotate(-hour_angle))
            self.tkminute = ImageTk.PhotoImage(self.minutehand.rotate(-minute_angle))
            self.tksecond = ImageTk.PhotoImage(self.secondhand.rotate(-second_angle))
            self.tkgear2 = ImageTk.PhotoImage(self.gear2.rotate(seconds * 5))
            self.tkgear3 = ImageTk.PhotoImage(self.gear3.rotate(seconds * 600))
            self.tkgear1 = ImageTk.PhotoImage(self.gear1.rotate(seconds * 10))
            self.lastUpdate = seconds

        self.canvas_gear1 = self.canvas.create_image(175, 170, image=self.tkgear1)
        self.canvas_gear2 = self.canvas.create_image(200, 220, image=self.tkgear2)
        self.canvas_gear3 = self.canvas.create_image(220, 190, image=self.tkgear3)
        self.canvas_gearcover = self.canvas.create_image(200, 200, image=self.gearcover)

        self.canvas_hour = self.canvas.create_image(200, 200, image=self.tkhour)
        self.canvas_minute = self.canvas.create_image(200, 200, image=self.tkminute)
        self.canvas_second = self.canvas.create_image(200, 200, image=self.tksecond)

    def fastUpdate(self):
        "Later"


class Interface:
    def __init__(self, root):
        self.analogClock = Clock(root)
        self.eventOutput = tk.Text(master=root)
        self.playerInput = tk.Entry(root, text="", bg="white", fg="black", bd=10)
        self.eventOutput.grid(row=0, column=0)
        self.playerInput.grid(row=1, column=0)
        self.analogClock.canvas.grid(row=0, column=1)
        root.bind("<Return>", playerAct)
        self.lastTick = 0
        self.analogClock.update()
        self.lastAct = 0
        self.gameTime = TIME.getTime("second")
        self.catchUpFactor = 0
        self.counter = 0
        self.update()

    def update(self):
        """catchUpFactor: number of game loops between ui updates"""
        self.realTime = time.perf_counter()
        if player.tAction > self.gameTime:
            if self.catchUpFactor == 0:
                "NUMBER OF GAME LOOPS THAT NEED TO HAPPEN/"
            self.catchUpFactor = (
                (player.tAction - self.gameTime) / (2 * pi) * (-cos(self.counter) + 1)
            )
            for i in range(floor(self.catchUpFactor)):
                self.gameLoop()
            self.counter += 0.1
        elif self.catchUpFactor:
            self.catchUpFactor = 0

        elif self.realTime - self.lastTick >= 1 / rate:
            self.gameLoop()

        self.analogClock.update()
        root.after(1, self.update)

    def gameLoop(self):
        # Advance time at prescribed rate or quickly advance to the next time the player can move.
        # Actions are updated at {resolution} of in game seconds
        # the interface is updated based on in-game ticks, defined in chrono.py and controlled by {rate} (FIX!: chrono.py should be moved to core, the ui should be moved elsewhere, and then rate can be defined with tick length)
        self.gameTime = TIME.tick()
        self.lastTick = self.realTime
        # Actor loop:
        if (self.gameTime - self.lastAct) > resolution:
            for actor in actors.actorList:
                if "user" in actor.tags:
                    result = actor.act()
                    if result:
                        self.report(player, result, verbose=VERBOSE)
                elif actor.tAction < self.gameTime:
                    result = actor.act()
                    self.report(player, result, verbose=VERBOSE)
                    actor.update()
            self.lastAct = self.gameTime

    """The rest of this class is for producing output."""



    # def getPronoun(noun):  # Not implemented
    #     if type(noun) is Thing:
    #         return "it"
    #     elif type(noun) is Actor:
    #         if noun.gender == "m":
    #             return "he"
    #         elif noun.gender == "f":
    #             return "she"
    #         else:
    #             return "they"

    # def conjugate(verb, actor):  # Not implemented
    #     if type(actor) is User:
    #         return verb.I

    def report(self, player, result, cue="visible", verbose=False):
        """Takes a result of the format [actor, verb, [targets], [locations], successBool, extraString]
        The actor performs the verb on the targets while in the locations, is either successful or not, and additional descriptive text can be provided.
        Cue is the manner in which information can get to the player.
        if verbose, sends all details to eventOutput for debugging
        otherwise determines if the player can see/hear the event, converts to human-readable format and outputs."""
        output = ""
        if verbose & (
            result[1] != "wait"
        ):  # Ignore 'wait' events to limit uninformative output
            output = str(result)
        else:
            if result[1] != "wait":
                if cue == "visible":
                    if self.outputVisible(player, result[3]):
                        output = self.actToText(player, result, "visible")
                if cue == "audible":
                    if self.outputAudible(player, result[3]):
                        output = self.actToText(player, result, "audible")
        if output:
            self.eventOutput.configure(state="normal")
            self.eventOutput.insert(tk.END, output)
            self.eventOutput.insert(tk.END, "\n")
            self.eventOutput.configure(state="disabled")
            self.eventOutput.see(tk.END)





def playerAct(event):
    player_move = parse(
        player, interface.playerInput.get()
    )  # returns move of format ['verb', 'object']
    print(interface.playerInput.get())
    interface.playerInput.delete(0, "end")
    if player_move:
        player.action = player_move

if __name__ == "__main__":
    # Set args
    VERBOSE = False
    rate = 5000  # ticks per second
    resolution = 1  # updates per second
    # Initialize world
   
    player = world2.worldInit()
    #player.tAction = TIME.getTime("second") + 80000
    # Produce TK environment
    root = tk.Tk()
    interface = Interface(root)
    root.mainloop()
