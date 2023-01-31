#!/bin/bash
echo "making sure computer is up to date";
sudo apt-get update;
echo "checking for git";
sudo apt-get install git-all;
echo "downloading material";
sudo git clone https://github.com/kernc/logkeys;
sudo apt install build-essential autotools-dev autoconf kbd;
echo "creating the drive";
cd logkeys;

sudo ./autogen.sh;

cd build;
 
sudo ../configure;

sudo make;

su -c "make install";

echo "Logkeys have been successfully installed";

echo "installing recording device now";

sudo sudo apt install recordmydesktop;

echo "Done installing all";
