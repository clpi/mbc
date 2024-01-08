#include <iostream>
using namespace std;

// int shell() {
//     char* line;
//     char** args;
//     int status = 1;
//     do {
//         cout << "> ";
//         line = read_line();
//     }
//     return status;
// }

int main(int const argc, char* const argv[]) {
    int image_w = 256;
    int image_h = 256;

    std::cout << "P3\n" << image_w << ' ' << image_h << "\n255\n";
    for (int j = 0; j < image_h; ++j) {
        for (int i = 0; i < image_w; ++i) {
            auto r = double(i) / (image_w - 1);
            auto g = double(j) / (image_h - 1);
            auto b = 0;

            int ir = static_cast<int>(255.999 * r);
            int ig = static_cast<int>(255.999 * g);
            int ib = static_cast<int>(255.999 * b);

            std::cout << ir << ' ' << ig << ' ' << ib << '\n';
        }
    }
}