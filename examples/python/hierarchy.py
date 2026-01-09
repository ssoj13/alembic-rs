#!/usr/bin/env python3
"""Hierarchy example: Building scene hierarchies.

This example demonstrates how to:
- Create nested transform hierarchies
- Build complex scene structures
- Navigate hierarchies during reading
"""

import alembic_rs
from alembic_rs import IArchive, OArchive


def create_cube_positions(size: float = 1.0):
    """Generate cube vertex positions."""
    s = size / 2
    return [
        [-s, -s, -s], [s, -s, -s], [s, s, -s], [-s, s, -s],
        [-s, -s,  s], [s, -s,  s], [s, s,  s], [-s, s,  s],
    ]


CUBE_FACE_COUNTS = [4, 4, 4, 4, 4, 4]
CUBE_FACE_INDICES = [
    0, 3, 2, 1, 4, 5, 6, 7, 0, 1, 5, 4,
    2, 3, 7, 6, 0, 4, 7, 3, 1, 2, 6, 5,
]


def write_solar_system(output_path: str) -> None:
    """Write a simple solar system hierarchy."""
    
    archive = OArchive.create(output_path)
    archive.setAppName("alembic_rs Hierarchy Example")
    archive.setDescription("Solar system hierarchy demo")
    
    # Root object
    root = alembic_rs.Abc.OObject("solar_system")
    
    # Sun at center
    sun_xform = alembic_rs.Abc.OXform("sun_xform")
    sun_xform.addIdentitySample()
    
    sun_mesh = alembic_rs.Abc.OPolyMesh("sun")
    sun_mesh.addSample(create_cube_positions(2.0), CUBE_FACE_COUNTS, CUBE_FACE_INDICES)
    sun_xform.addPolyMesh(sun_mesh)
    root.addXform(sun_xform)
    
    # Earth orbiting sun
    earth_orbit = alembic_rs.Abc.OXform("earth_orbit")
    earth_orbit.addTranslationSample(5.0, 0.0, 0.0)  # 5 units from sun
    
    earth_mesh = alembic_rs.Abc.OPolyMesh("earth")
    earth_mesh.addSample(create_cube_positions(0.5), CUBE_FACE_COUNTS, CUBE_FACE_INDICES)
    earth_orbit.addPolyMesh(earth_mesh)
    
    # Moon orbiting earth (nested transform)
    moon_orbit = alembic_rs.Abc.OXform("moon_orbit")
    moon_orbit.addTranslationSample(1.0, 0.0, 0.0)  # 1 unit from earth
    
    moon_mesh = alembic_rs.Abc.OPolyMesh("moon")
    moon_mesh.addSample(create_cube_positions(0.15), CUBE_FACE_COUNTS, CUBE_FACE_INDICES)
    moon_orbit.addPolyMesh(moon_mesh)
    
    earth_orbit.addXformChild(moon_orbit)
    root.addXform(earth_orbit)
    
    # Mars
    mars_orbit = alembic_rs.Abc.OXform("mars_orbit")
    mars_orbit.addTranslationSample(8.0, 0.0, 0.0)
    
    mars_mesh = alembic_rs.Abc.OPolyMesh("mars")
    mars_mesh.addSample(create_cube_positions(0.3), CUBE_FACE_COUNTS, CUBE_FACE_INDICES)
    mars_orbit.addPolyMesh(mars_mesh)
    root.addXform(mars_orbit)
    
    archive.writeArchive(root)
    archive.close()
    
    print(f"Written: {output_path}")


def write_robot_arm(output_path: str) -> None:
    """Write a hierarchical robot arm."""
    
    archive = OArchive.create(output_path)
    archive.setAppName("alembic_rs Hierarchy Example")
    
    root = alembic_rs.Abc.OObject("robot")
    
    # Base
    base = alembic_rs.Abc.OXform("base")
    base.addIdentitySample()
    
    base_mesh = alembic_rs.Abc.OPolyMesh("base_geo")
    base_mesh.addSample(create_cube_positions(1.0), CUBE_FACE_COUNTS, CUBE_FACE_INDICES)
    base.addPolyMesh(base_mesh)
    
    # Upper arm (child of base)
    upper_arm = alembic_rs.Abc.OXform("upper_arm")
    upper_arm.addTranslationSample(0.0, 1.0, 0.0)
    
    upper_arm_mesh = alembic_rs.Abc.OPolyMesh("upper_arm_geo")
    positions = [
        [-0.15, 0.0, -0.15], [0.15, 0.0, -0.15], [0.15, 2.0, -0.15], [-0.15, 2.0, -0.15],
        [-0.15, 0.0,  0.15], [0.15, 0.0,  0.15], [0.15, 2.0,  0.15], [-0.15, 2.0,  0.15],
    ]
    upper_arm_mesh.addSample(positions, CUBE_FACE_COUNTS, CUBE_FACE_INDICES)
    upper_arm.addPolyMesh(upper_arm_mesh)
    
    # Lower arm (child of upper arm)
    lower_arm = alembic_rs.Abc.OXform("lower_arm")
    lower_arm.addTranslationSample(0.0, 2.0, 0.0)
    
    lower_arm_mesh = alembic_rs.Abc.OPolyMesh("lower_arm_geo")
    positions = [
        [-0.1, 0.0, -0.1], [0.1, 0.0, -0.1], [0.1, 1.5, -0.1], [-0.1, 1.5, -0.1],
        [-0.1, 0.0,  0.1], [0.1, 0.0,  0.1], [0.1, 1.5,  0.1], [-0.1, 1.5,  0.1],
    ]
    lower_arm_mesh.addSample(positions, CUBE_FACE_COUNTS, CUBE_FACE_INDICES)
    lower_arm.addPolyMesh(lower_arm_mesh)
    
    # Gripper (child of lower arm)
    gripper = alembic_rs.Abc.OXform("gripper")
    gripper.addTranslationSample(0.0, 1.5, 0.0)
    
    gripper_mesh = alembic_rs.Abc.OPolyMesh("gripper_geo")
    gripper_mesh.addSample(create_cube_positions(0.2), CUBE_FACE_COUNTS, CUBE_FACE_INDICES)
    gripper.addPolyMesh(gripper_mesh)
    
    # Build hierarchy
    lower_arm.addXformChild(gripper)
    upper_arm.addXformChild(lower_arm)
    base.addXformChild(upper_arm)
    root.addXform(base)
    
    archive.writeArchive(root)
    archive.close()
    
    print(f"Written: {output_path}")


def print_hierarchy_tree(path: str) -> None:
    """Read and print full hierarchy tree."""
    
    archive = IArchive(path)
    top = archive.getTop()
    
    print(f"\n{archive.getName()}")
    print("=" * 50)
    
    def print_node(obj, indent=0):
        prefix = "  " * indent
        icon = get_icon(obj)
        print(f"{prefix}{icon} {obj.getName()}")
        
        # Print world transform if xform
        if obj.isXform():
            try:
                sample = obj.getXformSample(0)
                t = sample.getTranslation()
                print(f"{prefix}   pos: ({t[0]:.1f}, {t[1]:.1f}, {t[2]:.1f})")
            except:
                pass
        
        for i in range(obj.getNumChildren()):
            print_node(obj.getChild(i), indent + 1)
    
    def get_icon(obj):
        if obj.isXform():
            return "[X]"
        elif obj.isPolyMesh():
            return "[M]"
        elif obj.isCamera():
            return "[C]"
        else:
            return "[O]"
    
    print_node(top)


if __name__ == "__main__":
    import os
    
    output_dir = "output"
    os.makedirs(output_dir, exist_ok=True)
    
    solar_path = os.path.join(output_dir, "solar_system.abc")
    robot_path = os.path.join(output_dir, "robot_arm.abc")
    
    write_solar_system(solar_path)
    write_robot_arm(robot_path)
    
    print_hierarchy_tree(solar_path)
    print_hierarchy_tree(robot_path)
