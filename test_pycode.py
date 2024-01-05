fire = False

for position in enemy_positions:
    x_diff = position[0] - player_position[0]
    y_diff = position[1] - player_position[1]

    vec_from_player = [x_diff, y_diff]
    distance_from_player = (x_diff**2 + y_diff**2)**0.5

    vec_from_player = [x_diff / distance_from_player, y_diff / distance_from_player]
    
    player_heading = [player_position[2], player_position[3]]

    dot_product = vec_from_player[0] * player_heading[0] + vec_from_player[1] * player_heading[1]

    print(dot_product)

    if dot_product > 0.99:
        fire = True
