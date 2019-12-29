# To Do
  * Correct betterer headers
  * A CSS File
  * Custom logger for capturing logs
  * Add Distance Sensor Reading (i2c)
  * Don't blindly unwrap things. Handle the errors.
  * Evaluate changing the broadcast to shut down into a general broadcast to all tasks.
  * What is up with


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
