## Milton

([DEMO](https://www.youtube.com/watch?v=I8Yg-Q2ACvI)) A web app for controlling lights and accessing a camera feed
while remote.

| :camera: |
| --- |
| <img width="300px" src="https://user-images.githubusercontent.com/1545348/208220338-05c86e55-8296-41cd-9fa6-f7226e560a41.JPG" /> |


# Getting Started

Project layout:

```
/hardware              <- stl files for printer attachments
/src/milton-ui         <- elm frontend source code
/src/milton-web        <- rust web application
/src/milton-rs-lights  <- rust firmware for esp32c3 ws2812 led controller
```

> Before jumping in on the software side, it probably makes sense to get started printing the attachments and
ordering any components (to taste) that you'll need. Check out the [hardware readme](./hardware/README.md)
for more info.

## External Services: 

**Auth0**: This project takes advantage of [Auth0][auth0] as an oauth identity provider. They provide a free-tier of their
service for hobbyists/non-commercial use. This provides two benefits:

1. We don't necessarily need to concern ourselves (for the time being) with a user management database.
2. Adding support for additional identity provides can be handled within the Auth0 management portal.

To get 

**Redis**: Though our initial authentication is handled by Auth0, subsequent


# Hardware

See [`./hardware/README.md`](./hardware/README.md)


[auth0]: https://auth0.com/
