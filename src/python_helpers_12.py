class Ship():

    def __init__(self):
        self.velocity = player_velocity[0: 1]
        self.position = player_position[0: 1]
        self.heading_vector = player_position[2: 3]
        self.heading_angle = player_position[3: 4]

    def print_velocity(self):
        print(self.velocity)

    def foo(self):
        print("foo")

debug_list = []
def dbg(key = None, value = None):
    if value is None:
        return
        
    if key is None:
        debug_list.append(("KeylessDebug_", value))
    else:
        debug_list.append((key, value))
    



