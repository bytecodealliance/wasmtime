The original image of 000000062808.rgb is 000000062808.jpg from ImageNet
database. It processed by following Python code with
https://github.com/onnx/models/blob/bec48b6a70e5e9042c0badbaafefe4454e072d08/validated/vision/classification/imagenet_preprocess.py

```
image = mxnet.image.imread('000000062808.jpg')
image = preprocess_mxnet(image)
image.asnumpy().tofile('000000062808.rgb')
```