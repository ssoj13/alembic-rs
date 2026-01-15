// Test xform matrices using original Alembic C++ library
#include <Alembic/AbcGeom/All.h>
#include <Alembic/AbcCoreOgawa/All.h>
#include <iostream>
#include <iomanip>
#include <functional>

using namespace Alembic::AbcGeom;

void print_matrix(const Imath::M44d& m, const std::string& name) {
    std::cout << name << ":\n";
    for (int i = 0; i < 4; i++) {
        std::cout << "  [";
        for (int j = 0; j < 4; j++) {
            std::cout << std::fixed << std::setprecision(4) << std::setw(10) << m[i][j];
        }
        std::cout << "]\n";
    }
}

void dump_xform(IObject obj, const Imath::M44d& parent_world, int depth = 0) {
    std::string indent(depth * 2, ' ');
    
    if (IXform::matches(obj.getHeader())) {
        IXform xform(obj, kWrapExisting);
        IXformSchema& schema = xform.getSchema();
        XformSample sample;
        schema.get(sample, 0);
        
        Imath::M44d local = sample.getMatrix();
        bool inherits = sample.getInheritsXforms();
        
        Imath::M44d world;
        if (inherits) {
            world = local * parent_world;
        } else {
            world = local;
        }
        
        std::cout << indent << "[XFORM] " << obj.getName() 
                  << " (inherits=" << (inherits ? "true" : "false") << ")\n";
        
        // Print ops
        std::cout << indent << "  ops: " << sample.getNumOps() << "\n";
        for (size_t i = 0; i < sample.getNumOps(); i++) {
            XformOp op = sample.getOp(i);
            std::cout << indent << "    [" << i << "] type=" << op.getType() 
                      << " hint=" << (int)op.getHint() << " vals=[";
            for (size_t j = 0; j < op.getNumChannels(); j++) {
                if (j > 0) std::cout << ", ";
                std::cout << op.getChannelValue(j);
            }
            std::cout << "]\n";
        }
        
        // Print local matrix
        std::cout << indent << "  local matrix:\n";
        for (int i = 0; i < 4; i++) {
            std::cout << indent << "    [";
            for (int j = 0; j < 4; j++) {
                std::cout << std::fixed << std::setprecision(4) << std::setw(10) << local[i][j];
            }
            std::cout << "]\n";
        }
        
        // Print world matrix
        std::cout << indent << "  world matrix:\n";
        for (int i = 0; i < 4; i++) {
            std::cout << indent << "    [";
            for (int j = 0; j < 4; j++) {
                std::cout << std::fixed << std::setprecision(4) << std::setw(10) << world[i][j];
            }
            std::cout << "]\n";
        }
        std::cout << "\n";
        
        // Recurse children
        for (size_t i = 0; i < obj.getNumChildren(); i++) {
            dump_xform(obj.getChild(i), world, depth + 1);
        }
    } else {
        // Not xform, still check children
        for (size_t i = 0; i < obj.getNumChildren(); i++) {
            dump_xform(obj.getChild(i), parent_world, depth);
        }
    }
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <file.abc> [filter]\n";
        return 1;
    }
    
    std::string filter = argc > 2 ? argv[2] : "";
    
    IArchive archive(Alembic::AbcCoreOgawa::ReadArchive(), argv[1]);
    IObject root = archive.getTop();
    
    std::cout << "Archive: " << argv[1] << "\n";
    std::cout << "Filter: " << (filter.empty() ? "(none)" : filter) << "\n\n";
    
    Imath::M44d identity;
    identity.makeIdentity();
    
    // Find objects matching filter
    std::function<void(IObject, const Imath::M44d&, int)> find_and_dump;
    find_and_dump = [&](IObject obj, const Imath::M44d& parent_world, int depth) {
        Imath::M44d world = parent_world;
        
        if (IXform::matches(obj.getHeader())) {
            IXform xform(obj, kWrapExisting);
            XformSample sample;
            xform.getSchema().get(sample, 0);
            
            Imath::M44d local = sample.getMatrix();
            bool inherits = sample.getInheritsXforms();
            
            if (inherits) {
                world = local * parent_world;
            } else {
                world = local;
            }
            
            // Check if name matches filter
            if (filter.empty() || obj.getName().find(filter) != std::string::npos) {
                dump_xform(obj, parent_world, depth);
                return; // Don't recurse into children - dump_xform handles that
            }
        }
        
        // Recurse children
        for (size_t i = 0; i < obj.getNumChildren(); i++) {
            find_and_dump(obj.getChild(i), world, depth);
        }
    };
    
    find_and_dump(root, identity, 0);
    
    return 0;
}
