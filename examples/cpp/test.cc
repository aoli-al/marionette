#include <iostream>

int drop_in_place() {
  std::cout << "foo" << std::endl;
}

int main() {
  std::cout << "Print" << std::endl;
  drop_in_place();
}
