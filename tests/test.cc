#include <iostream>
#include <thread>
#include<unistd.h>

using namespace std;
bool lock;

int main() {
  auto f = [](int a) {

    lock = true;
    cout << "In lock " << a << endl;
    lock = false;
    cout << "Out lock " << a << endl;
  };
  thread t1(f, 1);
  thread t2(f, 2);
  thread t3(f, 3);
  t1.join();
  t2.join();
  t3.join();
}
