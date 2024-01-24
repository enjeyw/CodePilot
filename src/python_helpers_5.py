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



class Ship():

    def __init__(self):
        self.velocity = player_velocity[0: 1]
        self.position = player_position[0: 1]
        self.heading_vector = player_position[2: 3]
        self.heading_angle = player_position[3: 4]

    def print_velocity(self):
        print(self.velocity)

    def produce(self):
        print("produce")

    def p_foo(self):
        print("foo")

s = Ship()

s