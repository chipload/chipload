"""All tolerances live here. Do not write bare epsilons in algorithm code."""

CHAIN_TOL_MM = 0.001  # max gap when joining entities into a contour
ARC_APPROX_TOL_MM = 0.01  # max deviation when approximating splines/ellipses
ANGLE_TOL_RAD = 1e-9
