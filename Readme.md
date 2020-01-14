# To Do
  * Correct betterer headers
  * A CSS File
  * Custom logger for capturing logs
  * Don't blindly unwrap things. Handle the errors.
  * Use the checksum from the RFID reader
  * Configure the distance sensor
  * Configure the motor
  * Sanity in nameing the task functions
  * TOML Config

  ## Icon Url 
  [Cat Icon Origin](https://www.iconfinder.com/icons/3204662/animal_cat_domestic_pet_wild_icon)

  ## Setup Camera Cross Compiling

  ```sh
  $ sudo apt-get install libc6-armel-cross libc6-dev-armel-cross \
    binutils-arm-linux-gnueabi libncurses5-dev  
  $ rsync -r pi@ectopi:/opt/vc/* <target_dir>
  $ export MMAL_DIR=<target_dir>
  $ export MMAL_INCLUDE_DIR=$MMAL_DIR/include
  $ export MMAL_LIB_DIR=$MMAL_DIR/lib
  ```

## Installation

After compiling, move the binary to the desired directory.
Copy `cat-feeder.service` to the device and update paths where necessary.
Move `cat-feeder.service` to `/etc/systemd/system`

`sudo systemctl start cat-feeder.service`
`sudo systemctl stop cat-feeder.service`
