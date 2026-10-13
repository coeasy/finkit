#include <cmath>
#include <iostream>
#include <vector>
#include <finkit.h>

int main() {
    const double input[] = {1.0, 2.0, 3.0, 4.0};
    double output[4] = {};
    const auto status = ta_sma(input, output, 4, 3);
    if (status != 0 || !std::isnan(output[0]) || !std::isnan(output[1]) ||
        output[2] != 2.0 || output[3] != 3.0) {
        std::cerr << "installed C SDK SMA smoke failed, status=" << status << '\n';
        return 1;
    }
    std::cout << "installed C SDK SMA smoke passed\n";
    return 0;
}
