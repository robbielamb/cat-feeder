# To Do
  * Correct betterer headers
  * A CSS File
  * Custom logger for capturing logs
  * Don't blindly unwrap things. Handle the errors.
  * What is up with the pi zero?
  * Integrate the other build crap into the repository
  * Configure the distance sensor
  * Configure the motor
  * Sanity in nameing the task functions


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
